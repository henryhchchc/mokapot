//! Raw attribute structures as they appear in the JVM class-file bytecode.
//!
//! These types represent the binary layout of attributes before constant-pool
//! resolution. Each sub-module groups attributes by the JVM element they belong to.

mod annotation;
mod class;
mod code;
mod module;

pub use annotation::{Annotation, ElementValueInfo, TargetInfo, TypeAnnotation};
pub use class::{BootstrapMethod, EnclosingMethod, InnerClass, ParameterInfo, RecordComponentInfo};
pub use code::{
    Code, ExceptionTableEntry, LocalVariableInfo, StackMapFrameInfo, VerificationTypeInfo,
};
pub use module::{ExportsInfo, ModuleInfo, OpensInfo, ProvidesInfo, RequiresInfo};
