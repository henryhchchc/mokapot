//! Backend for generating LLVM IR for usage with tools provided by the LLVM
//! infrastructure.

use std::collections::HashMap;

use inkwell::{
    AddressSpace, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    context::{Context as LLVMContext, ContextRef},
    module::Module,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType},
    values::{BasicValueEnum, FunctionValue, IntValue, PointerValue},
};

use crate::{
    ir::{
        Identifier, MokaIRMethod, MokaInstruction, Operand,
        expression::{ArrayOperation, Condition, Expression, FieldAccess, MathOperation},
    },
    jvm::{
        ConstantValue, JavaString,
        code::{InstructionList, ProgramCounter},
        references::{FieldRef, MethodRef},
    },
    types::{
        Descriptor,
        field_type::{FieldType, PrimitiveType},
        method_descriptor::ReturnType,
    },
};
use utils::{get_or_insert_basic_block_ordered, upcast_to_u64};

mod intrinsics;
mod utils;

/// An error that occurs when lowering MokaIR to LLVM IR.
#[derive(Debug, thiserror::Error)]
pub enum LoweringError {
    /// The instruction is not yet supported by this backend.
    #[error("Unsupported MokaIR instruction: {0}")]
    UnsupportedInstruction(String),
    /// The expression is not yet supported by this backend.
    #[error("Unsupported MokaIR expression: {0}")]
    UnsupportedExpression(String),
    /// The constant is not yet supported by this backend.
    #[error("Unsupported constant value: {0}")]
    UnsupportedConstant(String),
    /// The operand is not yet supported by this backend.
    #[error("Unsupported operand: {0}")]
    UnsupportedOperand(String),
    /// The identifier has not been defined in the current lowering context.
    #[error("Undefined identifier during LLVM lowering: {0}")]
    UndefinedIdentifier(Identifier),
    /// LLVM rejected the generated module.
    #[error("LLVM verification failed: {0}")]
    VerificationFailed(String),
}

struct LoweringContext<'ctx, 'a> {
    ctx: ContextRef<'ctx>,
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    entry_builder: Builder<'ctx>,
    function: FunctionValue<'ctx>,
    instructions: &'a InstructionList<MokaInstruction>,
    slots: HashMap<Identifier, PointerValue<'ctx>>,
    slot_types: HashMap<Identifier, BasicTypeEnum<'ctx>>,
}

impl<'ctx, 'a> LoweringContext<'ctx, 'a> {
    fn new(
        ctx: ContextRef<'ctx>,
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
        function: FunctionValue<'ctx>,
        instructions: &'a InstructionList<MokaInstruction>,
    ) -> Self {
        let entry_builder = ctx.create_builder();
        let entry = function.get_first_basic_block().unwrap();
        if let Some(first_instruction) = entry.get_first_instruction() {
            entry_builder.position_before(&first_instruction);
        } else {
            entry_builder.position_at_end(entry);
        }
        Self {
            ctx,
            module,
            builder,
            entry_builder,
            function,
            instructions,
            slots: HashMap::new(),
            slot_types: HashMap::new(),
        }
    }

    fn create_slot(
        &mut self,
        identifier: Identifier,
        value_type: BasicTypeEnum<'ctx>,
    ) -> PointerValue<'ctx> {
        if let Some(slot) = self.slots.get(&identifier) {
            return *slot;
        }

        let entry = self.function.get_first_basic_block().unwrap();
        if let Some(first_instruction) = entry.get_first_instruction() {
            self.entry_builder.position_before(&first_instruction);
        } else {
            self.entry_builder.position_at_end(entry);
        }
        let slot = self
            .entry_builder
            .build_alloca(value_type, &sanitize_symbol(&identifier.to_string()))
            .unwrap();
        initialize_slot(&self.entry_builder, slot, value_type);
        self.slots.insert(identifier, slot);
        self.slot_types.insert(identifier, value_type);
        slot
    }

    fn store_identifier(&mut self, identifier: Identifier, value: BasicValueEnum<'ctx>) {
        let slot = self.create_slot(identifier, value.get_type());
        self.builder.build_store(slot, value).unwrap();
    }

    fn load_identifier(
        &mut self,
        identifier: Identifier,
    ) -> Result<BasicValueEnum<'ctx>, LoweringError> {
        let slot = self
            .slots
            .get(&identifier)
            .copied()
            .or_else(|| default_slot_for_identifier(self, identifier))
            .ok_or(LoweringError::UndefinedIdentifier(identifier))?;
        let slot_type = *self
            .slot_types
            .get(&identifier)
            .ok_or(LoweringError::UndefinedIdentifier(identifier))?;
        Ok(self
            .builder
            .build_load(slot_type, slot, &sanitize_symbol(&identifier.to_string()))
            .unwrap())
    }
}

/// Lowers a single method into a fresh LLVM module.
///
/// # Errors
/// Returns [`LoweringError`] if the method contains unsupported MokaIR or if LLVM verification
/// fails.
pub fn lower_method_to_module<'ctx>(
    llvm: &'ctx LLVMContext,
    module_name: &str,
    method: &MokaIRMethod,
) -> Result<Module<'ctx>, LoweringError> {
    lower_methods_to_module(llvm, module_name, [method])
}

/// Lowers multiple methods into a fresh LLVM module.
///
/// # Errors
/// Returns [`LoweringError`] if any method contains unsupported MokaIR or if LLVM verification
/// fails.
pub fn lower_methods_to_module<'ctx, 'm>(
    llvm: &'ctx LLVMContext,
    module_name: &str,
    methods: impl IntoIterator<Item = &'m MokaIRMethod>,
) -> Result<Module<'ctx>, LoweringError> {
    let module = llvm.create_module(module_name);
    for method in methods {
        lower_method(&module, method)?;
    }
    module
        .verify()
        .map_err(|err| LoweringError::VerificationFailed(err.to_string()))?;
    Ok(module)
}

/// Lowers a single method into an existing LLVM module.
///
/// # Errors
/// Returns [`LoweringError`] if the method contains unsupported MokaIR.
pub fn lower_method<'ctx>(
    module: &Module<'ctx>,
    method: &MokaIRMethod,
) -> Result<FunctionValue<'ctx>, LoweringError> {
    let ctx = module.get_context();
    let function = module.add_function(&method_symbol(method), function_type_of(ctx, method), None);
    let builder = ctx.create_builder();
    let entry = ctx.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    let mut lowering_ctx =
        LoweringContext::new(ctx, module, &builder, function, &method.instructions);
    bind_method_parameters(&mut lowering_ctx, method);

    for (&pc, instruction) in &method.instructions {
        instruction.lower(&mut lowering_ctx, pc)?;
    }

    if entry.get_terminator().is_none() {
        if let Some((&entry_pc, _)) = method.instructions.entry_point() {
            let entry_target = get_or_insert_basic_block_ordered(&lowering_ctx, function, entry_pc);
            builder.position_at_end(entry);
            builder.build_unconditional_branch(entry_target).unwrap();
        }
    }

    Ok(function)
}

fn bind_method_parameters<'ctx>(ctx: &mut LoweringContext<'ctx, '_>, method: &MokaIRMethod) {
    let mut parameter_index = 0_u32;

    if !method.is_static() {
        let this_param = ctx.function.get_nth_param(parameter_index).unwrap();
        this_param.set_name("this");
        let slot = ctx.create_slot(Identifier::This, this_param.get_type());
        ctx.builder.build_store(slot, this_param).unwrap();
        parameter_index += 1;
    }

    for argument_index in 0..method.descriptor.parameters_types.len() {
        let arg = ctx
            .function
            .get_nth_param(parameter_index + u32::try_from(argument_index).unwrap())
            .unwrap();
        arg.set_name(&format!("arg{argument_index}"));
        let identifier = Identifier::Arg(argument_index as u16);
        let slot = ctx.create_slot(identifier, arg.get_type());
        ctx.builder.build_store(slot, arg).unwrap();
    }
}

fn function_type_of<'ctx>(ctx: ContextRef<'ctx>, method: &MokaIRMethod) -> FunctionType<'ctx> {
    let mut parameter_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();

    if !method.is_static() {
        parameter_types.push(reference_type(ctx).into());
    }

    parameter_types.extend(
        method
            .descriptor
            .parameters_types
            .iter()
            .map(|field_type| BasicMetadataTypeEnum::from(llvm_type_of(ctx, field_type))),
    );

    match &method.descriptor.return_type {
        ReturnType::Void => ctx.void_type().fn_type(&parameter_types, false),
        ReturnType::Some(return_type) => {
            llvm_type_of(ctx, return_type).fn_type(&parameter_types, false)
        }
    }
}

fn llvm_type_of<'ctx>(ctx: ContextRef<'ctx>, field_type: &FieldType) -> BasicTypeEnum<'ctx> {
    match field_type {
        FieldType::Base(primitive) => primitive_type(ctx, *primitive).into(),
        FieldType::Object(_) | FieldType::Array(_) => reference_type(ctx).into(),
    }
}

fn primitive_type<'ctx>(ctx: ContextRef<'ctx>, primitive: PrimitiveType) -> BasicTypeEnum<'ctx> {
    match primitive {
        PrimitiveType::Boolean => ctx.bool_type().into(),
        PrimitiveType::Byte => ctx.i8_type().into(),
        PrimitiveType::Char | PrimitiveType::Short => ctx.i16_type().into(),
        PrimitiveType::Int => ctx.i32_type().into(),
        PrimitiveType::Long => ctx.i64_type().into(),
        PrimitiveType::Float => ctx.f32_type().into(),
        PrimitiveType::Double => ctx.f64_type().into(),
    }
}

fn reference_type<'ctx>(ctx: ContextRef<'ctx>) -> BasicTypeEnum<'ctx> {
    ctx.ptr_type(AddressSpace::default()).into()
}

fn method_symbol(method: &MokaIRMethod) -> String {
    format!(
        "{}_{}_{}",
        method.owner.binary_name,
        method.name,
        method.descriptor.descriptor()
    )
    .chars()
    .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
    .collect()
}

fn method_ref_symbol(method: &MethodRef) -> String {
    format!(
        "{}_{}_{}",
        method.owner.binary_name,
        method.name,
        method.descriptor.descriptor()
    )
    .chars()
    .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
    .collect()
}

fn field_ref_symbol(field: &FieldRef) -> String {
    format!(
        "{}_{}_{}",
        field.owner.binary_name,
        field.name,
        field.field_type.descriptor()
    )
    .chars()
    .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
    .collect()
}

fn sanitize_symbol(symbol: &str) -> String {
    symbol
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn initialize_slot<'ctx>(
    builder: &Builder<'ctx>,
    slot: PointerValue<'ctx>,
    value_type: BasicTypeEnum<'ctx>,
) {
    let zero_value: BasicValueEnum<'ctx> = match value_type {
        BasicTypeEnum::ArrayType(ty) => ty.const_zero().into(),
        BasicTypeEnum::FloatType(ty) => ty.const_zero().into(),
        BasicTypeEnum::IntType(ty) => ty.const_zero().into(),
        BasicTypeEnum::PointerType(ty) => ty.const_null().into(),
        BasicTypeEnum::StructType(ty) => ty.const_zero().into(),
        BasicTypeEnum::VectorType(ty) => ty.const_zero().into(),
        BasicTypeEnum::ScalableVectorType(ty) => ty.const_zero().into(),
    };
    builder.build_store(slot, zero_value).unwrap();
}

fn default_slot_for_identifier<'ctx>(
    ctx: &mut LoweringContext<'ctx, '_>,
    identifier: Identifier,
) -> Option<PointerValue<'ctx>> {
    let default_type = match identifier {
        Identifier::This | Identifier::CaughtException(_) => Some(reference_type(ctx.ctx)),
        Identifier::Arg(_) | Identifier::Local(_) => None,
    }?;
    Some(ctx.create_slot(identifier, default_type))
}

fn runtime_function<'ctx>(
    ctx: &LoweringContext<'ctx, '_>,
    name: &str,
    function_type: FunctionType<'ctx>,
) -> FunctionValue<'ctx> {
    ctx.module
        .get_function(name)
        .unwrap_or_else(|| ctx.module.add_function(name, function_type, None))
}

fn function_type_for_method_ref<'ctx>(
    ctx: ContextRef<'ctx>,
    method: &MethodRef,
    has_this: bool,
) -> FunctionType<'ctx> {
    let mut parameter_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
    if has_this {
        parameter_types.push(reference_type(ctx).into());
    }
    parameter_types.extend(
        method
            .descriptor
            .parameters_types
            .iter()
            .map(|field_type| BasicMetadataTypeEnum::from(llvm_type_of(ctx, field_type))),
    );
    match &method.descriptor.return_type {
        ReturnType::Void => ctx.void_type().fn_type(&parameter_types, false),
        ReturnType::Some(return_type) => {
            llvm_type_of(ctx, return_type).fn_type(&parameter_types, false)
        }
    }
}

fn placeholder_void_value<'ctx>(ctx: &LoweringContext<'ctx, '_>) -> BasicValueEnum<'ctx> {
    ctx.ctx.i8_type().const_zero().into()
}

fn lower_call_expression<'ctx>(
    ctx: &mut LoweringContext<'ctx, '_>,
    method: &MethodRef,
    this: Option<&Operand>,
    args: &[Operand],
    name: &str,
) -> Result<BasicValueEnum<'ctx>, LoweringError> {
    let has_this = this.is_some();
    let function_name = method_ref_symbol(method);
    let function = runtime_function(
        ctx,
        &function_name,
        function_type_for_method_ref(ctx.ctx, method, has_this),
    );

    let mut lowered_args = Vec::with_capacity(args.len() + usize::from(has_this));
    if let Some(this) = this {
        lowered_args.push(this.lower(ctx, ProgramCounter::ZERO)?.into());
    }
    for arg in args {
        lowered_args.push(arg.lower(ctx, ProgramCounter::ZERO)?.into());
    }

    let call = ctx
        .builder
        .build_call(function, &lowered_args, name)
        .unwrap();
    Ok(call
        .try_as_basic_value()
        .basic()
        .unwrap_or_else(|| placeholder_void_value(ctx)))
}

fn lower_closure_expression<'ctx>(
    ctx: &mut LoweringContext<'ctx, '_>,
    name: &str,
    captures: &[Operand],
) -> Result<BasicValueEnum<'ctx>, LoweringError> {
    let function_name = format!("mokapot_runtime_closure_{}", sanitize_symbol(name));
    let capture_values: Vec<BasicValueEnum<'ctx>> = captures
        .iter()
        .map(|capture| capture.lower(ctx, ProgramCounter::ZERO))
        .collect::<Result<_, _>>()?;
    let capture_types: Vec<_> = capture_values
        .iter()
        .map(|capture| BasicMetadataTypeEnum::from(capture.get_type()))
        .collect();
    let capture_values: Vec<_> = capture_values.into_iter().map(Into::into).collect();
    let function = runtime_function(
        ctx,
        &function_name,
        reference_type(ctx.ctx).fn_type(&capture_types, false),
    );
    let call = ctx
        .builder
        .build_call(function, &capture_values, "closure")
        .unwrap();
    Ok(call.try_as_basic_value().unwrap_basic())
}

fn string_global_ptr<'ctx>(
    ctx: &mut LoweringContext<'ctx, '_>,
    bytes: &[u8],
) -> BasicValueEnum<'ctx> {
    let symbol = format!(
        "mokapot_str_{}",
        sanitize_symbol(
            &bytes
                .iter()
                .map(|it| format!("{it:02x}"))
                .collect::<String>()
        )
    );
    let global = ctx.module.get_global(&symbol).unwrap_or_else(|| {
        let constant = ctx.ctx.const_string(bytes, true);
        let global =
            ctx.module
                .add_global(constant.get_type(), Some(AddressSpace::default()), &symbol);
        global.set_initializer(&constant);
        global.set_constant(true);
        global
    });
    global.as_pointer_value().into()
}

fn choose_phi_identifier(ids: &std::collections::HashSet<Identifier>) -> Option<Identifier> {
    let mut ids: Vec<_> = ids.iter().copied().collect();
    ids.sort_by_key(|identifier| phi_sort_key(*identifier));
    ids.into_iter().next()
}

fn phi_sort_key(identifier: Identifier) -> (u8, u16) {
    match identifier {
        Identifier::This => (0, 0),
        Identifier::Arg(index) => (1, index),
        Identifier::Local(local) => (2, local.into()),
        Identifier::CaughtException(pc) => (3, u16::from(pc)),
    }
}

/// Trait representing a struct that can be lowered into LLVM IR.
trait IRLowering {
    /// The type produced by the lowering operation.
    type Output<'ctx>;

    /// Lowers the LLVM IR representation of this struct and inserts it into the [`Module`].
    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError>;

    /// Lowers an equality operation and inserts it into the [`Builder`].
    fn lower_eq_op<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
        lhs: &Operand,
        rhs: &Operand,
        negated: bool,
    ) -> Result<IntValue<'ctx>, LoweringError> {
        let lhs = lhs.lower(ctx, pc)?;
        let rhs = rhs.lower(ctx, pc)?;

        let predicate = if negated {
            IntPredicate::NE
        } else {
            IntPredicate::EQ
        };

        match (lhs, rhs) {
            (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => Ok(ctx
                .builder
                .build_int_compare(predicate, lhs, rhs, "")
                .unwrap()),

            (BasicValueEnum::PointerValue(lhs), BasicValueEnum::PointerValue(rhs)) => {
                let lhs = ctx
                    .builder
                    .build_ptr_to_int(lhs, ctx.ctx.i64_type(), "")
                    .unwrap();
                let rhs = ctx
                    .builder
                    .build_ptr_to_int(rhs, ctx.ctx.i64_type(), "")
                    .unwrap();

                Ok(ctx
                    .builder
                    .build_int_compare(predicate, lhs, rhs, "")
                    .unwrap())
            }

            _ => Err(LoweringError::UnsupportedOperand(format!(
                "equality comparison between {lhs:?} and {rhs:?}"
            ))),
        }
    }

    /// Lowers an integer comparison operation and inserts it into the [`Builder`].
    fn lower_cmp_op<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
        lhs: &Operand,
        rhs: &Operand,
        llvm_cmpop: IntPredicate,
    ) -> Result<IntValue<'ctx>, LoweringError> {
        let BasicValueEnum::IntValue(lhs) = lhs.lower(ctx, pc)? else {
            return Err(LoweringError::UnsupportedOperand(format!(
                "comparison lhs {lhs}"
            )));
        };
        let BasicValueEnum::IntValue(rhs) = rhs.lower(ctx, pc)? else {
            return Err(LoweringError::UnsupportedOperand(format!(
                "comparison rhs {rhs}"
            )));
        };

        Ok(ctx
            .builder
            .build_int_compare(llvm_cmpop, lhs, rhs, "")
            .unwrap())
    }

    /// Lowers a null check and inserts it into the [`Builder`].
    fn lower_null_check<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
        operand: &Operand,
        negated: bool,
    ) -> Result<IntValue<'ctx>, LoweringError> {
        let BasicValueEnum::PointerValue(operand) = operand.lower(ctx, pc)? else {
            return Err(LoweringError::UnsupportedOperand(format!(
                "null check operand {operand}"
            )));
        };

        Ok(if negated {
            ctx.builder.build_is_not_null(operand, "").unwrap()
        } else {
            ctx.builder.build_is_null(operand, "").unwrap()
        })
    }

    /// Lowers a compare-with-zero operation and inserts it into the [`Builder`].
    ///
    /// Effectively inserts `{llvm_cmpop} {operand}, 0`.
    fn lower_cmp_zero_op<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
        operand: &Operand,
        llvm_cmpop: IntPredicate,
    ) -> Result<IntValue<'ctx>, LoweringError> {
        let BasicValueEnum::IntValue(operand) = operand.lower(ctx, pc)? else {
            return Err(LoweringError::UnsupportedOperand(format!(
                "zero comparison operand {operand}"
            )));
        };

        Ok(ctx
            .builder
            .build_int_compare(llvm_cmpop, operand, operand.get_type().const_zero(), "")
            .unwrap())
    }
}

impl IRLowering for MokaInstruction {
    type Output<'ctx> = ();

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        let func_val = ctx
            .builder
            .get_insert_block()
            .and_then(BasicBlock::get_parent)
            .unwrap();
        let this_bb = get_or_insert_basic_block_ordered(ctx, func_val, pc);

        if ctx
            .builder
            .get_insert_block()
            .and_then(BasicBlock::get_terminator)
            .is_none()
        {
            ctx.builder.build_unconditional_branch(this_bb).unwrap();
        }

        ctx.builder.position_at_end(this_bb);

        match self {
            MokaInstruction::Nop => intrinsics::invoke_donothing(ctx),
            MokaInstruction::Definition {
                value: destination,
                expr,
            } => {
                let lowered = expr.lower(ctx, pc)?;
                ctx.store_identifier((*destination).into(), lowered);
            }
            MokaInstruction::Jump { condition, target } => {
                let target_bb = get_or_insert_basic_block_ordered(ctx, func_val, *target);

                if let Some(condition) = condition {
                    let condition = condition.lower(ctx, pc)?;
                    let fallthrough_pc = ctx
                        .instructions
                        .next_pc_of(&pc)
                        .ok_or(LoweringError::UnsupportedInstruction(self.to_string()))?;
                    let fallthrough_bb =
                        get_or_insert_basic_block_ordered(ctx, func_val, fallthrough_pc);

                    ctx.builder
                        .build_conditional_branch(condition, target_bb, fallthrough_bb)
                        .unwrap();
                } else {
                    ctx.builder.build_unconditional_branch(target_bb).unwrap();
                }
            }

            MokaInstruction::Switch {
                match_value,
                branches,
                default,
            } => {
                let BasicValueEnum::IntValue(match_value) = match_value.lower(ctx, pc)? else {
                    return Err(LoweringError::UnsupportedOperand(self.to_string()));
                };
                let default_bb = get_or_insert_basic_block_ordered(ctx, func_val, *default);
                let cases: Vec<_> = branches
                    .iter()
                    .map(|(&key, &target)| {
                        (
                            ctx.ctx.i32_type().const_int(upcast_to_u64(key), true),
                            get_or_insert_basic_block_ordered(ctx, func_val, target),
                        )
                    })
                    .collect();
                ctx.builder
                    .build_switch(match_value, default_bb, &cases)
                    .unwrap();
            }

            MokaInstruction::Return(operand) => {
                if let Some(operand) = operand {
                    let operand = operand.lower(ctx, pc)?;
                    ctx.builder.build_return(Some(&operand)).unwrap();
                } else {
                    ctx.builder.build_return(None).unwrap();
                }
            }

            _ => {
                return Err(LoweringError::UnsupportedInstruction(self.to_string()));
            }
        }

        Ok(())
    }
}

impl IRLowering for Condition {
    type Output<'ctx> = IntValue<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            Condition::Equal(lhs, rhs) => self.lower_eq_op(ctx, pc, lhs, rhs, false),
            Condition::NotEqual(lhs, rhs) => self.lower_eq_op(ctx, pc, lhs, rhs, true),

            Condition::LessThan(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SLT)
            }
            Condition::LessThanOrEqual(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SLE)
            }
            Condition::GreaterThan(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SGT)
            }
            Condition::GreaterThanOrEqual(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SGE)
            }

            Condition::IsNull(operand) => self.lower_null_check(ctx, pc, operand, false),
            Condition::IsNotNull(operand) => self.lower_null_check(ctx, pc, operand, true),

            Condition::IsZero(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::EQ)
            }
            Condition::IsNonZero(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::NE)
            }
            Condition::IsPositive(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SGT)
            }
            Condition::IsNegative(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SLT)
            }
            Condition::IsNonNegative(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SGE)
            }
            Condition::IsNonPositive(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SLE)
            }
        }
    }
}

impl IRLowering for ConstantValue {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        _: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            ConstantValue::Null => Ok(ctx
                .ctx
                .ptr_type(AddressSpace::default())
                .const_null()
                .into()),
            ConstantValue::Integer(v) => {
                Ok(ctx.ctx.i32_type().const_int(upcast_to_u64(*v), true).into())
            }
            ConstantValue::Float(v) => Ok(ctx.ctx.f32_type().const_float(f64::from(*v)).into()),
            ConstantValue::Long(v) => {
                Ok(ctx.ctx.i64_type().const_int(upcast_to_u64(*v), true).into())
            }
            ConstantValue::Double(v) => Ok(ctx.ctx.f64_type().const_float(*v).into()),
            ConstantValue::String(JavaString::Utf8(value)) => {
                Ok(string_global_ptr(ctx, value.as_bytes()))
            }
            ConstantValue::String(JavaString::InvalidUtf8(bytes)) => {
                Ok(string_global_ptr(ctx, bytes))
            }
            _ => Err(LoweringError::UnsupportedConstant(self.to_string())),
        }
    }
}

impl IRLowering for Expression {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            Expression::Const(value) => value.lower(ctx, pc),
            Expression::Math(op) => op.lower(ctx, pc),
            Expression::Call { method, this, args } => {
                lower_call_expression(ctx, method, this.as_ref(), args, "call")
            }
            Expression::Closure { name, captures, .. } => {
                lower_closure_expression(ctx, name, captures)
            }
            Expression::Field(field_access) => field_access.lower(ctx, pc),
            Expression::Array(array_op) => array_op.lower(ctx, pc),
            _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
        }
    }
}

impl IRLowering for Identifier {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        _pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        ctx.load_identifier(*self)
    }
}

impl IRLowering for Operand {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            Operand::Just(identifier) => identifier.lower(ctx, pc),
            Operand::Phi(ids) => choose_phi_identifier(ids)
                .ok_or(LoweringError::UnsupportedOperand(self.to_string()))?
                .lower(ctx, pc),
        }
    }
}

impl IRLowering for FieldAccess {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            FieldAccess::ReadStatic { field } => {
                let function_name =
                    format!("mokapot_runtime_get_static_{}", field_ref_symbol(field));
                let function = runtime_function(
                    ctx,
                    &function_name,
                    llvm_type_of(ctx.ctx, &field.field_type).fn_type(&[], false),
                );
                let call = ctx
                    .builder
                    .build_call(function, &[], "read_static")
                    .unwrap();
                Ok(call
                    .try_as_basic_value()
                    .basic()
                    .unwrap_or_else(|| placeholder_void_value(ctx)))
            }
            FieldAccess::WriteStatic { field, value } => {
                let value = value.lower(ctx, pc)?;
                let function_name =
                    format!("mokapot_runtime_put_static_{}", field_ref_symbol(field));
                let function = runtime_function(
                    ctx,
                    &function_name,
                    ctx.ctx
                        .void_type()
                        .fn_type(&[value.get_type().into()], false),
                );
                ctx.builder
                    .build_call(function, &[value.into()], "")
                    .unwrap();
                Ok(placeholder_void_value(ctx))
            }
            FieldAccess::ReadInstance { object_ref, field } => {
                let object_ref = object_ref.lower(ctx, pc)?;
                let function_name =
                    format!("mokapot_runtime_get_field_{}", field_ref_symbol(field));
                let function = runtime_function(
                    ctx,
                    &function_name,
                    llvm_type_of(ctx.ctx, &field.field_type)
                        .fn_type(&[reference_type(ctx.ctx).into()], false),
                );
                let call = ctx
                    .builder
                    .build_call(function, &[object_ref.into()], "read_field")
                    .unwrap();
                Ok(call
                    .try_as_basic_value()
                    .basic()
                    .unwrap_or_else(|| placeholder_void_value(ctx)))
            }
            FieldAccess::WriteInstance {
                object_ref,
                field,
                value,
            } => {
                let object_ref = object_ref.lower(ctx, pc)?;
                let value = value.lower(ctx, pc)?;
                let function_name =
                    format!("mokapot_runtime_put_field_{}", field_ref_symbol(field));
                let function = runtime_function(
                    ctx,
                    &function_name,
                    ctx.ctx.void_type().fn_type(
                        &[reference_type(ctx.ctx).into(), value.get_type().into()],
                        false,
                    ),
                );
                ctx.builder
                    .build_call(function, &[object_ref.into(), value.into()], "")
                    .unwrap();
                Ok(placeholder_void_value(ctx))
            }
        }
    }
}

impl IRLowering for ArrayOperation {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            ArrayOperation::New {
                element_type,
                length,
            } => {
                let BasicValueEnum::IntValue(length) = length.lower(ctx, pc)? else {
                    return Err(LoweringError::UnsupportedOperand(self.to_string()));
                };
                let function_name = format!(
                    "mokapot_runtime_new_array_{}",
                    sanitize_symbol(&element_type.descriptor())
                );
                let function = runtime_function(
                    ctx,
                    &function_name,
                    reference_type(ctx.ctx).fn_type(&[length.get_type().into()], false),
                );
                let call = ctx
                    .builder
                    .build_call(function, &[length.into()], "new_array")
                    .unwrap();
                Ok(call.try_as_basic_value().unwrap_basic())
            }
            ArrayOperation::Read { array_ref, index } => {
                let array_ref = array_ref.lower(ctx, pc)?;
                let BasicValueEnum::IntValue(index) = index.lower(ctx, pc)? else {
                    return Err(LoweringError::UnsupportedOperand(self.to_string()));
                };
                let function = runtime_function(
                    ctx,
                    "mokapot_runtime_array_read_i32",
                    ctx.ctx.i32_type().fn_type(
                        &[reference_type(ctx.ctx).into(), index.get_type().into()],
                        false,
                    ),
                );
                let call = ctx
                    .builder
                    .build_call(function, &[array_ref.into(), index.into()], "array_read")
                    .unwrap();
                Ok(call.try_as_basic_value().unwrap_basic())
            }
            ArrayOperation::Write {
                array_ref,
                index,
                value,
            } => {
                let array_ref = array_ref.lower(ctx, pc)?;
                let BasicValueEnum::IntValue(index) = index.lower(ctx, pc)? else {
                    return Err(LoweringError::UnsupportedOperand(self.to_string()));
                };
                let value = value.lower(ctx, pc)?;
                let function = runtime_function(
                    ctx,
                    "mokapot_runtime_array_write_i32",
                    reference_type(ctx.ctx).fn_type(
                        &[
                            reference_type(ctx.ctx).into(),
                            index.get_type().into(),
                            value.get_type().into(),
                        ],
                        false,
                    ),
                );
                let call = ctx
                    .builder
                    .build_call(
                        function,
                        &[array_ref.into(), index.into(), value.into()],
                        "array_write",
                    )
                    .unwrap();
                Ok(call
                    .try_as_basic_value()
                    .basic()
                    .unwrap_or_else(|| array_ref))
            }
            ArrayOperation::Length { array_ref } => {
                let array_ref = array_ref.lower(ctx, pc)?;
                let function = runtime_function(
                    ctx,
                    "mokapot_runtime_array_length",
                    ctx.ctx
                        .i32_type()
                        .fn_type(&[reference_type(ctx.ctx).into()], false),
                );
                let call = ctx
                    .builder
                    .build_call(function, &[array_ref.into()], "array_length")
                    .unwrap();
                Ok(call.try_as_basic_value().unwrap_basic())
            }
            ArrayOperation::NewMultiDim { .. } => {
                Err(LoweringError::UnsupportedExpression(self.to_string()))
            }
        }
    }
}

impl IRLowering for MathOperation {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    #[allow(clippy::too_many_lines)]
    fn lower<'ctx>(
        &self,
        ctx: &mut LoweringContext<'ctx, '_>,
        pc: ProgramCounter,
    ) -> Result<Self::Output<'ctx>, LoweringError> {
        match self {
            MathOperation::Add(lhs, rhs) => {
                let lhs = lhs.lower(ctx, pc)?;
                let rhs = rhs.lower(ctx, pc)?;

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                        Ok(ctx.builder.build_int_add(lhs, rhs, "").unwrap().into())
                    }
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        Ok(ctx.builder.build_float_add(lhs, rhs, "").unwrap().into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            MathOperation::Subtract(lhs, rhs) => {
                let lhs = lhs.lower(ctx, pc)?;
                let rhs = rhs.lower(ctx, pc)?;

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                        Ok(ctx.builder.build_int_sub(lhs, rhs, "").unwrap().into())
                    }
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        Ok(ctx.builder.build_float_sub(lhs, rhs, "").unwrap().into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            MathOperation::Multiply(lhs, rhs) => {
                let lhs = lhs.lower(ctx, pc)?;
                let rhs = rhs.lower(ctx, pc)?;

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                        Ok(ctx.builder.build_int_mul(lhs, rhs, "").unwrap().into())
                    }
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        Ok(ctx.builder.build_float_mul(lhs, rhs, "").unwrap().into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            MathOperation::Divide(lhs, rhs) => {
                let lhs = lhs.lower(ctx, pc)?;
                let rhs = rhs.lower(ctx, pc)?;

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => Ok(ctx
                        .builder
                        .build_int_signed_div(lhs, rhs, "")
                        .unwrap()
                        .into()),
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        Ok(ctx.builder.build_float_div(lhs, rhs, "").unwrap().into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            MathOperation::Remainder(lhs, rhs) => {
                let lhs = lhs.lower(ctx, pc)?;
                let rhs = rhs.lower(ctx, pc)?;

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => Ok(ctx
                        .builder
                        .build_int_signed_rem(lhs, rhs, "")
                        .unwrap()
                        .into()),
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        Ok(ctx.builder.build_float_rem(lhs, rhs, "").unwrap().into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            MathOperation::Negate(value) => {
                let value = value.lower(ctx, pc)?;

                match value {
                    BasicValueEnum::IntValue(value) => {
                        Ok(ctx.builder.build_int_neg(value, "").unwrap().into())
                    }
                    BasicValueEnum::FloatValue(value) => {
                        Ok(ctx.builder.build_float_neg(value, "").unwrap().into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            MathOperation::Increment(value, constant) => {
                let value = value.lower(ctx, pc)?;

                match value {
                    BasicValueEnum::IntValue(value) => {
                        let constant = value.get_type().const_int(upcast_to_u64(*constant), true);
                        Ok(ctx
                            .builder
                            .build_int_add(value, constant, "")
                            .unwrap()
                            .into())
                    }
                    BasicValueEnum::FloatValue(value) => {
                        let constant = value.get_type().const_float(f64::from(*constant));
                        Ok(ctx
                            .builder
                            .build_float_add(value, constant, "")
                            .unwrap()
                            .into())
                    }
                    _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
                }
            }

            _ => Err(LoweringError::UnsupportedExpression(self.to_string())),
        }
    }
}
