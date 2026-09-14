//! Lifts stack-based JVM instructions into register-based instructions.

mod arrays;
mod calls;
mod control_flow;
pub(super) mod fallibility;
mod fields;
mod numeric;
mod operations;
pub(super) mod successors;
mod values;
mod wide;

use crate::{
    ir::{
        expression::{Condition, Conversion, LockOperation, MathOperation, NaNTreatment},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                NodeAddress,
                frame::{CATEGORY_1, CATEGORY_2, Frame, JvmFrameError},
                instruction::RegisterInstruction,
                symbolic_execution::{Executor, Value},
            },
        },
    },
    jvm::{
        ConstantValue,
        code::{Instruction as JVM, ProgramCounter},
    },
    types::{field_type::FieldType, method_descriptor::ReturnType},
};

pub(super) struct LiftContext<'executor, 'frame, 'method> {
    executor: &'executor mut Executor<'method>,
    location: NodeAddress,
    pc: ProgramCounter,
    frame: &'frame mut Frame<Value>,
}

impl Executor<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "the match is an exhaustive JVM instruction dispatch"
    )]
    pub(super) fn lift_register_instruction(
        &mut self,
        jvm_instruction: &JVM,
        location: NodeAddress,
        frame: &mut Frame<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        #[allow(
            clippy::enum_glob_use,
            reason = "this match exhaustively dispatches the JVM instruction enum"
        )]
        use JVM::*;

        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let mut cx = LiftContext {
            executor: self,
            location,
            pc,
            frame,
        };

        match jvm_instruction {
            AConstNull => cx.constant::<CATEGORY_1>(ConstantValue::Null),
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
                let value = i32::from(jvm_instruction.opcode()) - i32::from(IConst0.opcode());
                let constant = ConstantValue::Integer(value);
                cx.constant::<CATEGORY_1>(constant)
            }
            LConst0 | LConst1 => {
                let value = i64::from(jvm_instruction.opcode()) - i64::from(LConst0.opcode());
                let constant = ConstantValue::Long(value);
                cx.constant::<CATEGORY_2>(constant)
            }
            FConst0 | FConst1 | FConst2 => {
                let value = f32::from(jvm_instruction.opcode()) - f32::from(FConst0.opcode());
                let constant = ConstantValue::Float(value);
                cx.constant::<CATEGORY_1>(constant)
            }
            DConst0 | DConst1 => {
                let value = f64::from(jvm_instruction.opcode()) - f64::from(DConst0.opcode());
                let constant = ConstantValue::Double(value);
                cx.constant::<CATEGORY_2>(constant)
            }
            BiPush(value) => cx.constant::<CATEGORY_1>(ConstantValue::Integer(i32::from(*value))),
            SiPush(value) => cx.constant::<CATEGORY_1>(ConstantValue::Integer(i32::from(*value))),
            Ldc(value) | LdcW(value) => cx.constant::<CATEGORY_1>(value.clone()),
            Ldc2W(value) => cx.constant::<CATEGORY_2>(value.clone()),
            ILoad(idx) | FLoad(idx) | ALoad(idx) => cx.load::<CATEGORY_1>((*idx).into()),
            LLoad(idx) | DLoad(idx) => cx.load::<CATEGORY_2>((*idx).into()),
            ILoad0 | FLoad0 | ALoad0 => cx.load::<CATEGORY_1>(0),
            ILoad1 | FLoad1 | ALoad1 => cx.load::<CATEGORY_1>(1),
            ILoad2 | FLoad2 | ALoad2 => cx.load::<CATEGORY_1>(2),
            ILoad3 | FLoad3 | ALoad3 => cx.load::<CATEGORY_1>(3),
            LLoad0 | DLoad0 => cx.load::<CATEGORY_2>(0),
            LLoad1 | DLoad1 => cx.load::<CATEGORY_2>(1),
            LLoad2 | DLoad2 => cx.load::<CATEGORY_2>(2),
            LLoad3 | DLoad3 => cx.load::<CATEGORY_2>(3),
            IStore(idx) | FStore(idx) | AStore(idx) => cx.store::<CATEGORY_1>((*idx).into()),
            LStore(idx) | DStore(idx) => cx.store::<CATEGORY_2>((*idx).into()),
            IStore0 | FStore0 | AStore0 => cx.store::<CATEGORY_1>(0),
            IStore1 | FStore1 | AStore1 => cx.store::<CATEGORY_1>(1),
            IStore2 | FStore2 | AStore2 => cx.store::<CATEGORY_1>(2),
            IStore3 | FStore3 | AStore3 => cx.store::<CATEGORY_1>(3),
            LStore0 | DStore0 => cx.store::<CATEGORY_2>(0),
            LStore1 | DStore1 => cx.store::<CATEGORY_2>(1),
            LStore2 | DStore2 => cx.store::<CATEGORY_2>(2),
            LStore3 | DStore3 => cx.store::<CATEGORY_2>(3),
            IInc(idx, constant) => cx.increment((*idx).into(), *constant),
            Wide(wide) => cx.lift_wide(wide),
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => cx.array_read::<CATEGORY_1>(),
            LALoad | DALoad => cx.array_read::<CATEGORY_2>(),
            IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
                cx.array_write::<CATEGORY_1>()
            }
            LAStore | DAStore => cx.array_write::<CATEGORY_2>(),
            ANewArray(element_type) => cx.new_array(element_type.clone().into()),
            NewArray(element_type) => cx.new_array(FieldType::Base(*element_type)),
            MultiANewArray(element_type, dimension) => {
                cx.new_multi_array(element_type.clone().into(), *dimension)
            }
            ArrayLength => cx.array_length(),
            Pop => cx.stack_effect(Frame::pop),
            Pop2 => cx.stack_effect(Frame::pop2),
            Dup => cx.stack_effect(Frame::dup),
            DupX1 => cx.stack_effect(Frame::dup_x1),
            DupX2 => cx.stack_effect(Frame::dup_x2),
            Swap => cx.stack_effect(Frame::swap),
            Dup2 => cx.stack_effect(Frame::dup2),
            Dup2X1 => cx.stack_effect(Frame::dup2_x1),
            Dup2X2 => cx.stack_effect(Frame::dup2_x2),
            IAdd | FAdd => cx.binary::<CATEGORY_1>(MathOperation::Add),
            ISub | FSub => cx.binary::<CATEGORY_1>(MathOperation::Subtract),
            IMul | FMul => cx.binary::<CATEGORY_1>(MathOperation::Multiply),
            IDiv | FDiv => cx.binary::<CATEGORY_1>(MathOperation::Divide),
            IRem | FRem => cx.binary::<CATEGORY_1>(MathOperation::Remainder),
            LAdd | DAdd => cx.binary::<CATEGORY_2>(MathOperation::Add),
            LSub | DSub => cx.binary::<CATEGORY_2>(MathOperation::Subtract),
            LMul | DMul => cx.binary::<CATEGORY_2>(MathOperation::Multiply),
            LDiv | DDiv => cx.binary::<CATEGORY_2>(MathOperation::Divide),
            LRem | DRem => cx.binary::<CATEGORY_2>(MathOperation::Remainder),
            INeg | FNeg => cx.unary::<CATEGORY_1>(MathOperation::Negate),
            LNeg | DNeg => cx.unary::<CATEGORY_2>(MathOperation::Negate),
            IShl => cx.binary::<CATEGORY_1>(MathOperation::ShiftLeft),
            IShr => cx.binary::<CATEGORY_1>(MathOperation::ShiftRight),
            IUShr => cx.binary::<CATEGORY_1>(MathOperation::LogicalShiftRight),
            LShl => cx.shift_long(MathOperation::ShiftLeft),
            LShr => cx.shift_long(MathOperation::ShiftRight),
            LUShr => cx.shift_long(MathOperation::LogicalShiftRight),
            IAnd => cx.binary::<CATEGORY_1>(MathOperation::BitwiseAnd),
            IOr => cx.binary::<CATEGORY_1>(MathOperation::BitwiseOr),
            IXor => cx.binary::<CATEGORY_1>(MathOperation::BitwiseXor),
            LAnd => cx.binary::<CATEGORY_2>(MathOperation::BitwiseAnd),
            LOr => cx.binary::<CATEGORY_2>(MathOperation::BitwiseOr),
            LXor => cx.binary::<CATEGORY_2>(MathOperation::BitwiseXor),
            I2F => cx.conversion::<CATEGORY_1, CATEGORY_1>(Conversion::Int2Float),
            I2L => cx.conversion::<CATEGORY_1, CATEGORY_2>(Conversion::Int2Long),
            I2D => cx.conversion::<CATEGORY_1, CATEGORY_2>(Conversion::Int2Double),
            L2I => cx.conversion::<CATEGORY_2, CATEGORY_1>(Conversion::Long2Int),
            L2F => cx.conversion::<CATEGORY_2, CATEGORY_1>(Conversion::Long2Float),
            L2D => cx.conversion::<CATEGORY_2, CATEGORY_2>(Conversion::Long2Double),
            F2I => cx.conversion::<CATEGORY_1, CATEGORY_1>(Conversion::Float2Int),
            F2L => cx.conversion::<CATEGORY_1, CATEGORY_2>(Conversion::Float2Long),
            F2D => cx.conversion::<CATEGORY_1, CATEGORY_2>(Conversion::Float2Double),
            D2I => cx.conversion::<CATEGORY_2, CATEGORY_1>(Conversion::Double2Int),
            D2L => cx.conversion::<CATEGORY_2, CATEGORY_2>(Conversion::Double2Long),
            D2F => cx.conversion::<CATEGORY_2, CATEGORY_1>(Conversion::Double2Float),
            I2B => cx.conversion::<CATEGORY_1, CATEGORY_1>(Conversion::Int2Byte),
            I2C => cx.conversion::<CATEGORY_1, CATEGORY_1>(Conversion::Int2Char),
            I2S => cx.conversion::<CATEGORY_1, CATEGORY_1>(Conversion::Int2Short),
            LCmp => cx.compare_long(),
            FCmpL => cx.compare_float::<CATEGORY_1>(NaNTreatment::IsSmallest),
            FCmpG => cx.compare_float::<CATEGORY_1>(NaNTreatment::IsLargest),
            DCmpL => cx.compare_float::<CATEGORY_2>(NaNTreatment::IsSmallest),
            DCmpG => cx.compare_float::<CATEGORY_2>(NaNTreatment::IsLargest),
            IfEq(target) => cx.unary_branch(*target, Condition::IsZero),
            IfNe(target) => cx.unary_branch(*target, Condition::IsNonZero),
            IfLt(target) => cx.unary_branch(*target, Condition::IsNegative),
            IfGe(target) => cx.unary_branch(*target, Condition::IsNonNegative),
            IfGt(target) => cx.unary_branch(*target, Condition::IsPositive),
            IfLe(target) => cx.unary_branch(*target, Condition::IsNonPositive),
            IfNull(target) => cx.unary_branch(*target, Condition::IsNull),
            IfNonNull(target) => cx.unary_branch(*target, Condition::IsNotNull),
            IfICmpEq(target) | IfACmpEq(target) => cx.comparison_branch(*target, Condition::Equal),
            IfICmpNe(target) | IfACmpNe(target) => {
                cx.comparison_branch(*target, Condition::NotEqual)
            }
            IfICmpGe(target) => cx.comparison_branch(*target, Condition::GreaterThanOrEqual),
            IfICmpLt(target) => cx.comparison_branch(*target, Condition::LessThan),
            IfICmpGt(target) => cx.comparison_branch(*target, Condition::GreaterThan),
            IfICmpLe(target) => cx.comparison_branch(*target, Condition::LessThanOrEqual),
            Goto(target) | GotoW(target) => Ok(RegisterInstruction::Jump {
                condition: None,
                target: *target,
            }),
            Jsr(target) | JsrW(target) => cx.subroutine_call(*target),
            Ret(idx) => cx.subroutine_return((*idx).into()),
            TableSwitch {
                range,
                jump_targets,
                default,
            } => cx.switch(*default, range.clone().zip(jump_targets.clone()).collect()),
            LookupSwitch {
                default,
                match_targets,
            } => cx.switch(*default, match_targets.clone()),
            IReturn | FReturn | AReturn => cx.return_value::<CATEGORY_1>(),
            LReturn | DReturn => cx.return_value::<CATEGORY_2>(),
            Return => Ok(RegisterInstruction::Return(None)),
            AThrow => cx.throw(),
            GetStatic(field) => cx.read_static(field),
            GetField(field) => cx.read_instance(field),
            PutStatic(field) => cx.write_static(field),
            PutField(field) => cx.write_instance(field),
            InvokeVirtual(method) | InvokeSpecial(method) | InvokeInterface(method, _) => {
                cx.invoke(method, true)
            }
            InvokeStatic(method) => cx.invoke(method, false),
            InvokeDynamic {
                bootstrap_method_index,
                name,
                descriptor,
            } => cx.invoke_dynamic(descriptor, *bootstrap_method_index, name),
            Nop | Breakpoint | ImpDep1 | ImpDep2 => Ok(RegisterInstruction::Erased),
            New(class) => cx.new_object(class),
            CheckCast(target) => cx.conversion::<CATEGORY_1, CATEGORY_1>(|value| {
                Conversion::CheckCast(value, target.clone())
            }),
            InstanceOf(target) => cx.conversion::<CATEGORY_1, CATEGORY_1>(|value| {
                Conversion::InstanceOf(value, target.clone())
            }),
            MonitorEnter => cx.monitor(LockOperation::Acquire),
            MonitorExit => cx.monitor(LockOperation::Release),
        }
    }
}

impl LiftContext<'_, '_, '_> {
    fn monitor(
        &mut self,
        operation: impl FnOnce(Value) -> LockOperation<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let object_ref = self.frame.pop_value::<CATEGORY_1>()?;
        Ok(RegisterInstruction::Effect(operation(object_ref).into()))
    }

    fn stack_effect<F>(&mut self, effect: F) -> Result<RegisterInstruction, MokaIRBuildError>
    where
        F: FnOnce(&mut Frame<Value>) -> Result<(), JvmFrameError>,
    {
        effect(self.frame)?;
        Ok(RegisterInstruction::Erased)
    }

    fn definition_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        self.executor.definition_id_at(self.location)
    }

    fn with_def<T, L>(&mut self, lift: L) -> Result<T, MokaIRBuildError>
    where
        L: FnOnce(SsaValueId, &mut Frame<Value>) -> Result<T, MokaIRBuildError>,
    {
        let value = self.definition_id()?;
        lift(value, self.frame)
    }

    fn unary<const SLOT: bool>(
        &mut self,
        operation: impl FnOnce(Value) -> MathOperation<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        self.with_def(|value, frame| {
            let operand = frame.pop_value::<SLOT>()?;
            frame.push_value::<SLOT>(value.into())?;
            let expr = operation(operand).into();
            Ok(RegisterInstruction::Definition { value, expr })
        })
    }

    fn binary<const SLOT: bool>(
        &mut self,
        operation: impl FnOnce(Value, Value) -> MathOperation<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        self.with_def(|value, frame| operations::lift_binary_math::<SLOT>(frame, value, operation))
    }

    fn conversion<const OPERAND_SLOT: bool, const RESULT_SLOT: bool>(
        &mut self,
        conversion: impl FnOnce(Value) -> Conversion<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        self.with_def(|value, frame| {
            operations::lift_conversion::<OPERAND_SLOT, RESULT_SLOT>(frame, value, conversion)
        })
    }

    fn definition_id_for_return(
        &mut self,
        return_type: &ReturnType,
    ) -> Result<Option<SsaValueId>, MokaIRBuildError> {
        match return_type {
            ReturnType::Some(_) => self.definition_id().map(Some),
            ReturnType::Void => Ok(None),
        }
    }
}
