//! Generic type signitures in the JVM
//! This module is work in progress, and the signatures that are not yet implemented are aliased to [`String`].
//!
#![doc = see_jvm_spec!(4, 7, 9, 1)]

use crate::macros::see_jvm_spec;

/// A generic type signature for a class.
pub type ClassSignature = String;

/// A generic type signature for a method.
pub type MethodSignature = String;

/// A generic type signature for a field, a formal parameter, a local variable, or a record component.
pub type FieldSignature = String;
