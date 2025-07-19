use std::fmt::{self, Display, Formatter};
use mokapot::jvm::{class, field, method, Class, Field, Method};
use mokapot::types::method_descriptor;

/// Formatter for classes that implements Display
pub struct ClassFormatter<'a> {
    class: &'a Class,
}

impl<'a> ClassFormatter<'a> {
    pub fn new(class: &'a Class) -> Self {
        Self { class }
    }

    /// Format class access flags into a human-readable string
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

/// Formatter for fields that implements Display
pub struct FieldFormatter<'a> {
    field: &'a Field,
}

impl<'a> FieldFormatter<'a> {
    pub fn new(field: &'a Field) -> Self {
        Self { field }
    }

    /// Format field access flags into a human-readable string
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

impl<'a> Display for FieldFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let access_flags = self.format_access_flags();
        if !access_flags.is_empty() {
            write!(f, "{access_flags} ")?;
        }
        write!(f, "{} {}", self.field.field_type, self.field.name)?;

        if let Some(value) = &self.field.constant_value {
            write!(f, " = {value}")?;
        }
        write!(f, ";")
    }
}

/// Formatter for methods that implements Display
pub struct MethodFormatter<'a> {
    method: &'a Method,
}

impl<'a> MethodFormatter<'a> {
    pub fn new(method: &'a Method) -> Self {
        Self { method }
    }

    /// Format method access flags into a human-readable string
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

impl<'a> Display for MethodFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let access_flags = self.format_access_flags();
        if !access_flags.is_empty() {
            write!(f, "{access_flags} ")?;
        }

        let return_type = match &self.method.descriptor.return_type {
            method_descriptor::ReturnType::Some(field_type) => field_type.to_string(),
            method_descriptor::ReturnType::Void => "void".to_string(),
        };

        write!(f, "{return_type} {}(", self.method.name)?;

        for (i, param_type) in self.method.descriptor.parameters_types.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{param_type}")?;
        }
        write!(f, ");")
    }
}
