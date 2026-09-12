use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            analysis::analyzer::JvmFrameAnalyzer,
            lifting::semantics::JvmSemantics,
            normalization::{Location, ReturnAddress},
        },
    },
    jvm::code::{MethodBody, ProgramCounter},
};

impl JvmSemantics for JvmFrameAnalyzer<'_> {
    fn body(&self) -> &MethodBody {
        self.body
    }

    fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Bytecode { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.definition_ids.get(&location) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.definition_ids.insert(location, id);
        Ok(id)
    }

    fn caught_exception_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Handler { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.caught_exception_ids.get(&location) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.caught_exception_ids.insert(location, id);
        Ok(id)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(self.next_pc_of(pc)?, context)
    }

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(target, context)
    }

    fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.handler(handler, context)
    }

    fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError> {
        self.normalizer.register(Location::Unwind)
    }

    fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        self.normalizer.enter(location, target, continuation)
    }

    fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.normalizer.return_from(location, address)
    }
}
