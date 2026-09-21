//! Lifts stack-based JVM instructions into register-form instructions.

mod arrays;
mod calls;
mod fields;
mod operations;
mod values;
mod wide;

use ValueCategory::{Category1, Category2};

use super::{Frame, StackOperation, ValueCategory, values::ValueContext};
use crate::{
    ir::{
        Operation, ValueId,
        expression::{Conversion, Expression, LockOperation, MathOperation, NaNTreatment},
        generator::error::Error,
    },
    jvm::{
        ConstantValue,
        code::{Instruction as JVM, ProgramCounter},
    },
    types::field_type::FieldType,
};

/// Builds the definition operation produced by a lifted expression.
const fn definition_operation(value: ValueId, expr: Expression) -> Operation {
    Operation::Definition { value, expr }
}

pub(super) struct LiftContext<'values, 'frame> {
    values: &'values mut ValueContext,
    pc: ProgramCounter,
    frame: &'frame mut Frame,
}

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive JVM instruction dispatch"
)]
pub(super) fn lift_instruction(
    values: &mut ValueContext,
    jvm_instruction: &JVM,
    pc: ProgramCounter,
    frame: &mut Frame,
) -> Result<Option<Operation>, Error> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this match exhaustively dispatches the JVM instruction enum"
    )]
    use JVM::*;

    let mut cx = LiftContext { values, pc, frame };

    match jvm_instruction {
        AConstNull => cx.constant(ConstantValue::Null, Category1),
        IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
            let value = i32::from(jvm_instruction.opcode()) - i32::from(IConst0.opcode());
            let constant = ConstantValue::Integer(value);
            cx.constant(constant, Category1)
        }
        LConst0 | LConst1 => {
            let value = i64::from(jvm_instruction.opcode()) - i64::from(LConst0.opcode());
            let constant = ConstantValue::Long(value);
            cx.constant(constant, Category2)
        }
        FConst0 | FConst1 | FConst2 => {
            let value = f32::from(jvm_instruction.opcode()) - f32::from(FConst0.opcode());
            let constant = ConstantValue::Float(value);
            cx.constant(constant, Category1)
        }
        DConst0 | DConst1 => {
            let value = f64::from(jvm_instruction.opcode()) - f64::from(DConst0.opcode());
            let constant = ConstantValue::Double(value);
            cx.constant(constant, Category2)
        }
        BiPush(value) => cx.constant(ConstantValue::Integer(i32::from(*value)), Category1),
        SiPush(value) => cx.constant(ConstantValue::Integer(i32::from(*value)), Category1),
        Ldc(value) | LdcW(value) => cx.constant(value.clone(), Category1),
        Ldc2W(value) => cx.constant(value.clone(), Category2),
        ILoad(idx) | FLoad(idx) | ALoad(idx) => cx.load((*idx).into(), Category1),
        LLoad(idx) | DLoad(idx) => cx.load((*idx).into(), Category2),
        ILoad0 | FLoad0 | ALoad0 => cx.load(0, Category1),
        ILoad1 | FLoad1 | ALoad1 => cx.load(1, Category1),
        ILoad2 | FLoad2 | ALoad2 => cx.load(2, Category1),
        ILoad3 | FLoad3 | ALoad3 => cx.load(3, Category1),
        LLoad0 | DLoad0 => cx.load(0, Category2),
        LLoad1 | DLoad1 => cx.load(1, Category2),
        LLoad2 | DLoad2 => cx.load(2, Category2),
        LLoad3 | DLoad3 => cx.load(3, Category2),
        IStore(idx) | FStore(idx) | AStore(idx) => cx.store((*idx).into(), Category1),
        LStore(idx) | DStore(idx) => cx.store((*idx).into(), Category2),
        IStore0 | FStore0 | AStore0 => cx.store(0, Category1),
        IStore1 | FStore1 | AStore1 => cx.store(1, Category1),
        IStore2 | FStore2 | AStore2 => cx.store(2, Category1),
        IStore3 | FStore3 | AStore3 => cx.store(3, Category1),
        LStore0 | DStore0 => cx.store(0, Category2),
        LStore1 | DStore1 => cx.store(1, Category2),
        LStore2 | DStore2 => cx.store(2, Category2),
        LStore3 | DStore3 => cx.store(3, Category2),
        IInc(idx, constant) => cx.increment((*idx).into(), *constant),
        Wide(wide) => cx.lift_wide(wide),
        IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => cx.array_read(Category1),
        LALoad | DALoad => cx.array_read(Category2),
        IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => cx.array_write(Category1),
        LAStore | DAStore => cx.array_write(Category2),
        ANewArray(element_type) => cx.new_array(element_type.clone().into()),
        NewArray(element_type) => cx.new_array(FieldType::Base(*element_type)),
        MultiANewArray(element_type, dimension) => {
            cx.new_multi_array(element_type.clone().into(), *dimension)
        }
        ArrayLength => cx.array_length(),
        Pop => cx.stack_effect(StackOperation::Pop),
        Pop2 => cx.stack_effect(StackOperation::Pop2),
        Dup => cx.stack_effect(StackOperation::Dup),
        DupX1 => cx.stack_effect(StackOperation::DupX1),
        DupX2 => cx.stack_effect(StackOperation::DupX2),
        Swap => cx.stack_effect(StackOperation::Swap),
        Dup2 => cx.stack_effect(StackOperation::Dup2),
        Dup2X1 => cx.stack_effect(StackOperation::Dup2X1),
        Dup2X2 => cx.stack_effect(StackOperation::Dup2X2),
        IAdd | FAdd => cx.binary(MathOperation::Add, Category1),
        ISub | FSub => cx.binary(MathOperation::Subtract, Category1),
        IMul | FMul => cx.binary(MathOperation::Multiply, Category1),
        IDiv | FDiv => cx.binary(MathOperation::Divide, Category1),
        IRem | FRem => cx.binary(MathOperation::Remainder, Category1),
        LAdd | DAdd => cx.binary(MathOperation::Add, Category2),
        LSub | DSub => cx.binary(MathOperation::Subtract, Category2),
        LMul | DMul => cx.binary(MathOperation::Multiply, Category2),
        LDiv | DDiv => cx.binary(MathOperation::Divide, Category2),
        LRem | DRem => cx.binary(MathOperation::Remainder, Category2),
        INeg | FNeg => cx.unary(MathOperation::Negate, Category1),
        LNeg | DNeg => cx.unary(MathOperation::Negate, Category2),
        IShl => cx.binary(MathOperation::ShiftLeft, Category1),
        IShr => cx.binary(MathOperation::ShiftRight, Category1),
        IUShr => cx.binary(MathOperation::LogicalShiftRight, Category1),
        LShl => cx.shift_long(MathOperation::ShiftLeft),
        LShr => cx.shift_long(MathOperation::ShiftRight),
        LUShr => cx.shift_long(MathOperation::LogicalShiftRight),
        IAnd => cx.binary(MathOperation::BitwiseAnd, Category1),
        IOr => cx.binary(MathOperation::BitwiseOr, Category1),
        IXor => cx.binary(MathOperation::BitwiseXor, Category1),
        LAnd => cx.binary(MathOperation::BitwiseAnd, Category2),
        LOr => cx.binary(MathOperation::BitwiseOr, Category2),
        LXor => cx.binary(MathOperation::BitwiseXor, Category2),
        I2F => cx.conversion(Conversion::Int2Float, Category1, Category1),
        I2L => cx.conversion(Conversion::Int2Long, Category1, Category2),
        I2D => cx.conversion(Conversion::Int2Double, Category1, Category2),
        L2I => cx.conversion(Conversion::Long2Int, Category2, Category1),
        L2F => cx.conversion(Conversion::Long2Float, Category2, Category1),
        L2D => cx.conversion(Conversion::Long2Double, Category2, Category2),
        F2I => cx.conversion(Conversion::Float2Int, Category1, Category1),
        F2L => cx.conversion(Conversion::Float2Long, Category1, Category2),
        F2D => cx.conversion(Conversion::Float2Double, Category1, Category2),
        D2I => cx.conversion(Conversion::Double2Int, Category2, Category1),
        D2L => cx.conversion(Conversion::Double2Long, Category2, Category2),
        D2F => cx.conversion(Conversion::Double2Float, Category2, Category1),
        I2B => cx.conversion(Conversion::Int2Byte, Category1, Category1),
        I2C => cx.conversion(Conversion::Int2Char, Category1, Category1),
        I2S => cx.conversion(Conversion::Int2Short, Category1, Category1),
        LCmp => cx.compare_long(),
        FCmpL => cx.compare_float(NaNTreatment::IsSmallest, Category1),
        FCmpG => cx.compare_float(NaNTreatment::IsLargest, Category1),
        DCmpL => cx.compare_float(NaNTreatment::IsSmallest, Category2),
        DCmpG => cx.compare_float(NaNTreatment::IsLargest, Category2),
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
        Nop | Breakpoint | ImpDep1 | ImpDep2 => Ok(None),
        New(class) => cx.new_object(class),
        CheckCast(target) => cx.conversion(
            |value| Conversion::CheckCast(value, target.clone()),
            Category1,
            Category1,
        ),
        InstanceOf(target) => cx.conversion(
            |value| Conversion::InstanceOf(value, target.clone()),
            Category1,
            Category1,
        ),
        MonitorEnter => cx.monitor(LockOperation::Acquire),
        MonitorExit => cx.monitor(LockOperation::Release),
        // All the remainders are control flow instructions.
        _ => panic!("control-transfer instruction reached non-control lifting"),
    }
}

impl LiftContext<'_, '_> {
    fn monitor(
        &mut self,
        operation: impl FnOnce(ValueId) -> LockOperation,
    ) -> Result<Option<Operation>, Error> {
        let object_ref = self.frame.stack.pop(Category1)?;
        let expr = operation(object_ref).into();
        Ok(Some(Operation::Effect { expr }))
    }

    fn stack_effect(&mut self, operation: StackOperation) -> Result<Option<Operation>, Error> {
        self.frame.stack.apply(operation)?;
        Ok(None)
    }

    fn definition_id(&mut self) -> ValueId {
        self.values.definition_at(self.pc)
    }

    fn with_def<T, L>(&mut self, lift: L) -> Result<T, Error>
    where
        L: FnOnce(ValueId, &mut Frame) -> Result<T, Error>,
    {
        let value = self.definition_id();
        lift(value, self.frame)
    }

    fn unary(
        &mut self,
        operation: impl FnOnce(ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        self.with_def(|value, frame| {
            let operand = frame.stack.pop(category)?;
            frame.stack.push(value, category)?;
            let expr = operation(operand).into();
            Ok(Some(definition_operation(value, expr)))
        })
    }

    fn binary(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        self.with_def(|value, frame| {
            operations::lift_binary_math(frame, value, operation, category)
        })
    }

    fn conversion(
        &mut self,
        conversion: impl FnOnce(ValueId) -> Conversion,
        operand_category: ValueCategory,
        result_category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        self.with_def(|value, frame| {
            operations::lift_conversion(frame, value, conversion, operand_category, result_category)
        })
    }
}
