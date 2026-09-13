use crate::ir::generator::jvm::frame::{error::JvmFrameError, stack_frame::Frame};

impl<V: Clone> Frame<V> {
    pub fn pop(&mut self) -> Result<(), JvmFrameError> {
        let _top_element = self.pop_slot()?;
        Ok(())
    }

    pub fn pop2(&mut self) -> Result<(), JvmFrameError> {
        let _top_element = self.pop_slot()?;
        let _top_element = self.pop_slot()?;
        Ok(())
    }

    pub fn dup(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        self.push_slot(top_element.clone())?;
        self.push_slot(top_element)?;
        Ok(())
    }

    pub fn dup_x1(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        let second_element = self.pop_slot()?;
        self.push_slot(top_element.clone())?;
        self.push_slot(second_element)?;
        self.push_slot(top_element)?;
        Ok(())
    }

    pub fn dup_x2(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        let second_element = self.pop_slot()?;
        let third_element = self.pop_slot()?;
        self.push_slot(top_element.clone())?;
        self.push_slot(third_element)?;
        self.push_slot(second_element)?;
        self.push_slot(top_element)?;
        Ok(())
    }

    pub fn dup2(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        let second_element = self.pop_slot()?;
        self.push_slot(second_element.clone())?;
        self.push_slot(top_element.clone())?;
        self.push_slot(second_element)?;
        self.push_slot(top_element)?;
        Ok(())
    }

    pub fn dup2_x1(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        let second_element = self.pop_slot()?;
        let third_element = self.pop_slot()?;
        self.push_slot(second_element.clone())?;
        self.push_slot(top_element.clone())?;
        self.push_slot(third_element)?;
        self.push_slot(second_element)?;
        self.push_slot(top_element)?;
        Ok(())
    }

    pub fn dup2_x2(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        let second_element = self.pop_slot()?;
        let third_element = self.pop_slot()?;
        let fourth_element = self.pop_slot()?;
        self.push_slot(second_element.clone())?;
        self.push_slot(top_element.clone())?;
        self.push_slot(fourth_element)?;
        self.push_slot(third_element)?;
        self.push_slot(second_element)?;
        self.push_slot(top_element)?;
        Ok(())
    }

    pub fn swap(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_slot()?;
        let second_element = self.pop_slot()?;
        self.push_slot(top_element)?;
        self.push_slot(second_element)?;
        Ok(())
    }
}
