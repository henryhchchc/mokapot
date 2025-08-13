use super::{error::ExecutionError, stack_frame::JvmStackFrame};

/// Stack manipulation operations for JVM frames
pub(crate) trait StackOperations {
    fn pop(&mut self) -> Result<(), ExecutionError>;
    fn pop2(&mut self) -> Result<(), ExecutionError>;
    fn dup(&mut self) -> Result<(), ExecutionError>;
    fn dup_x1(&mut self) -> Result<(), ExecutionError>;
    fn dup_x2(&mut self) -> Result<(), ExecutionError>;
    fn dup2(&mut self) -> Result<(), ExecutionError>;
    fn dup2_x1(&mut self) -> Result<(), ExecutionError>;
    fn dup2_x2(&mut self) -> Result<(), ExecutionError>;
    fn swap(&mut self) -> Result<(), ExecutionError>;
}

impl StackOperations for JvmStackFrame {
    fn pop(&mut self) -> Result<(), ExecutionError> {
        let _top_element = self.pop_raw()?;
        Ok(())
    }

    fn pop2(&mut self) -> Result<(), ExecutionError> {
        let _top_element = self.pop_raw()?;
        let _top_element = self.pop_raw()?;
        Ok(())
    }

    fn dup(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(top_element)?;
        Ok(())
    }

    fn dup_x1(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    fn dup_x2(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    fn dup2(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    fn dup2_x1(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    fn dup2_x2(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        let fourth_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(fourth_element)?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    fn swap(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(top_element)?;
        self.push_raw(second_element)?;
        Ok(())
    }
}
