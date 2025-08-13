//! Formatters for Java class components
//!
//! This module provides formatter types that implement the Display trait for
//! different Java class file components. These formatters convert the internal
//! representation of classes, fields, and methods into human-readable text
//! similar to what the javap tool would produce.

use std::fmt::{self, Display, Formatter};

use mokapot::{
    jvm::{Class, Field, Method, class, field, method},
    types::method_descriptor,
};

/// Formatter for Java classes
///
/// Provides Display implementation for Class structures, allowing them to be
/// formatted as text. This formatter handles the class name, access flags,
/// and other class-level attributes.
pub struct ClassFormatter<'a> {
    class: &'a Class,
}

impl<'a> ClassFormatter<'a> {
    /// Creates a new class formatter for the specified class
    ///
    /// # Arguments
    ///
    /// * `class` - Reference to the Class to format
    pub fn new(class: &'a Class) -> Self {
        Self { class }
    }

    /// Format class access flags into a human-readable string
    ///
    /// Converts JVM access flags (stored as bitflags) into human-readable
    /// keywords like "public", "final", "abstract", etc., following
    /// the same format as the Java `javap` tool.
    ///
    /// # Returns
    ///
    /// A space-separated string of access flag keywords
    pub fn format_access_flags(&self) -> String {
        let flags = self.class.access_flags;
        let mut result = Vec::new();

        if flags.contains(class::AccessFlags::PUBLIC) {
            result.push("public");
        }
        if flags.contains(class::AccessFlags::FINAL) {
            result.push("final");
        }
        if flags.contains(class::AccessFlags::SUPER) {
            // Not displayed in javap output
        }
        if flags.contains(class::AccessFlags::INTERFACE) {
            result.push("interface");
        }
        if flags.contains(class::AccessFlags::ABSTRACT) {
            result.push("abstract");
        }
        if flags.contains(class::AccessFlags::SYNTHETIC) {
            result.push("synthetic");
        }
        if flags.contains(class::AccessFlags::ANNOTATION) {
            result.push("annotation");
        }
        if flags.contains(class::AccessFlags::ENUM) {
            result.push("enum");
        }

        result.join(" ")
    }
}

/// Formatter for Java fields
///
/// Provides Display implementation for Field structures, allowing them to be
/// formatted as text. This formatter handles field declarations including
/// access modifiers, type, name, and constant values if present.
pub struct FieldFormatter<'a> {
    field: &'a Field,
}

impl<'a> FieldFormatter<'a> {
    /// Creates a new field formatter for the specified field
    ///
    /// # Arguments
    ///
    /// * `field` - Reference to the Field to format
    pub fn new(field: &'a Field) -> Self {
        Self { field }
    }

    /// Format field access flags into a human-readable string
    ///
    /// Converts JVM access flags (stored as bitflags) into human-readable
    /// keywords like "public", "static", "final", etc., following
    /// the same format as the Java `javap` tool.
    ///
    /// # Returns
    ///
    /// A space-separated string of access flag keywords
    pub fn format_access_flags(&self) -> String {
        let flags = self.field.access_flags;
        let mut result = Vec::new();

        if flags.contains(field::AccessFlags::PUBLIC) {
            result.push("public");
        }
        if flags.contains(field::AccessFlags::PRIVATE) {
            result.push("private");
        }
        if flags.contains(field::AccessFlags::PROTECTED) {
            result.push("protected");
        }
        if flags.contains(field::AccessFlags::STATIC) {
            result.push("static");
        }
        if flags.contains(field::AccessFlags::FINAL) {
            result.push("final");
        }
        if flags.contains(field::AccessFlags::VOLATILE) {
            result.push("volatile");
        }
        if flags.contains(field::AccessFlags::TRANSIENT) {
            result.push("transient");
        }
        if flags.contains(field::AccessFlags::SYNTHETIC) {
            result.push("synthetic");
        }
        if flags.contains(field::AccessFlags::ENUM) {
            result.push("enum");
        }

        result.join(" ")
    }
}

/// Implements Display for FieldFormatter to enable string conversion
///
/// The Display implementation follows Java source syntax:
/// - Access modifiers first (public, private, etc.)
/// - Field type and name
/// - Constant value if present (for static final fields)
/// - Ending with a semicolon
impl<'a> Display for FieldFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // First print access flags if present
        let access_flags = self.format_access_flags();
        if !access_flags.is_empty() {
            write!(f, "{access_flags} ")?;
        }

        // Print field type and name
        write!(f, "{} {}", self.field.field_type, self.field.name)?;

        // For constant fields, print their value
        if let Some(value) = &self.field.constant_value {
            write!(f, " = {value}")?;
        }

        // End with semicolon
        write!(f, ";")
    }
}

/// Formatter for Java methods
///
/// Provides Display implementation for Method structures, allowing them to be
/// formatted as text. This formatter handles method declarations including
/// access modifiers, return type, name, and parameters.
pub struct MethodFormatter<'a> {
    method: &'a Method,
}

impl<'a> MethodFormatter<'a> {
    /// Creates a new method formatter for the specified method
    ///
    /// # Arguments
    ///
    /// * `method` - Reference to the Method to format
    pub fn new(method: &'a Method) -> Self {
        Self { method }
    }

    /// Format method access flags into a human-readable string
    ///
    /// Converts JVM access flags (stored as bitflags) into human-readable
    /// keywords like "public", "static", "synchronized", etc., following
    /// the same format as the Java `javap` tool.
    ///
    /// # Returns
    ///
    /// A space-separated string of access flag keywords
    pub fn format_access_flags(&self) -> String {
        let flags = self.method.access_flags;
        let mut result = Vec::new();

        if flags.contains(method::AccessFlags::PUBLIC) {
            result.push("public");
        }
        if flags.contains(method::AccessFlags::PRIVATE) {
            result.push("private");
        }
        if flags.contains(method::AccessFlags::PROTECTED) {
            result.push("protected");
        }
        if flags.contains(method::AccessFlags::STATIC) {
            result.push("static");
        }
        if flags.contains(method::AccessFlags::FINAL) {
            result.push("final");
        }
        if flags.contains(method::AccessFlags::SYNCHRONIZED) {
            result.push("synchronized");
        }
        if flags.contains(method::AccessFlags::BRIDGE) {
            result.push("bridge");
        }
        if flags.contains(method::AccessFlags::VARARGS) {
            result.push("varargs");
        }
        if flags.contains(method::AccessFlags::NATIVE) {
            result.push("native");
        }
        if flags.contains(method::AccessFlags::ABSTRACT) {
            result.push("abstract");
        }
        if flags.contains(method::AccessFlags::STRICT) {
            result.push("strictfp");
        }
        if flags.contains(method::AccessFlags::SYNTHETIC) {
            result.push("synthetic");
        }

        result.join(" ")
    }
}

/// Implements Display for MethodFormatter to enable string conversion
///
/// The Display implementation follows Java source syntax:
/// - Access modifiers first (public, static, etc.)
/// - Return type
/// - Method name
/// - Parameter list in parentheses
/// - Ending with a semicolon
impl<'a> Display for MethodFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // First print access flags if present
        let access_flags = self.format_access_flags();
        if !access_flags.is_empty() {
            write!(f, "{access_flags} ")?;
        }

        // Convert the method's return type descriptor to a string
        // A return type can either be a specific type or void
        let return_type = match &self.method.descriptor.return_type {
            method_descriptor::ReturnType::Some(field_type) => field_type.to_string(),
            method_descriptor::ReturnType::Void => "void".to_string(),
        };

        // Write the return type and method name
        write!(f, "{return_type} {}(", self.method.name)?;

        // Write the parameter list, comma-separated
        for (i, param_type) in self.method.descriptor.parameters_types.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{param_type}")?;
        }

        // Close the parameter list and end with semicolon
        write!(f, ");")
    }
}
