//! Generic type signitures in the JVM
//! This module is work in progress, and the signatures that are not yet implemented are aliased to [`String`].
//!
//! See the [JVM Specification §4.7.9.1](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.9.1) for more information.

/// A generic type signature for a class.
pub type ClassSignature = String;

/// A generic type signature for a method.
pub type MethodSignature = String;

/// A generic type signature for a field, a formal parameter, a local variable, or a record component.
pub type FieldSignature = String;
