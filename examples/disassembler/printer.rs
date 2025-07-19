use mokapot::jvm::{Class, Field, Method};
use mokapot::jvm::code::MethodBody;

use crate::formatters::{ClassFormatter, FieldFormatter, MethodFormatter};

/// Structure for printing class information with consistent formatting
pub struct ClassPrinter<'a> {
    class: &'a Class,
    verbose: bool,
}

impl<'a> ClassPrinter<'a> {
    /// Create a new class printer for the given class
    pub fn new(class: &'a Class, verbose: bool) -> Self {
        Self { class, verbose }
    }

    /// Print all class information
    pub fn print(&self) {
        self.print_header();

        if self.verbose {
            self.print_details();
        }

        println!();
        println!("{} {{", self.class.binary_name);

        // Print fields
        for field in &self.class.fields {
            self.print_field(field);
        }

        println!();

        // Print methods
        for method in &self.class.methods {
            self.print_method(method);
        }

        println!("}}");
    }

    /// Print the class header information
    fn print_header(&self) {
        if self.verbose {
            println!("Compiled from \"Unknown source\"");
        }

        let formatter = ClassFormatter::new(self.class);
        let access_flags = formatter.format_access_flags();

        if access_flags.is_empty() {
            println!("class {}", self.class.binary_name);
        } else {
            println!("{} class {}", access_flags, self.class.binary_name);
        }

        if let Some(super_class) = &self.class.super_class {
            println!("  extends {}", super_class.binary_name);
        }

        if !self.class.interfaces.is_empty() {
            print!("  implements ");
            let mut interfaces = self.class.interfaces.iter().peekable();
            while let Some(interface) = interfaces.next() {
                print!("{}", interface.binary_name);
                if interfaces.peek().is_some() {
                    print!(", ");
                }
            }
            println!();
        }
    }

    /// Print detailed class information (for verbose mode)
    fn print_details(&self) {
        println!("  minor version: {}", self.class.version.minor());
        println!("  major version: {}", self.class.version.major());
        println!("  flags: {:#06x}", self.class.access_flags.bits());
    }

    /// Print information about a field
    fn print_field(&self, field: &Field) {
        println!("  {}", FieldFormatter::new(field));

        if self.verbose {
            self.print_field_details(field);
        }
    }

    /// Print additional field details in verbose mode
    fn print_field_details(&self, field: &Field) {
        if field.is_synthetic {
            println!("    Synthetic: true");
        }
        if field.is_deprecated {
            println!("    Deprecated: true");
        }
        if !field.runtime_visible_annotations.is_empty() {
            println!("    Runtime visible annotations: {}", field.runtime_visible_annotations.len());
        }
        if !field.runtime_invisible_annotations.is_empty() {
            println!("    Runtime invisible annotations: {}", field.runtime_invisible_annotations.len());
        }
    }

    /// Print information about a method
    fn print_method(&self, method: &Method) {
        println!("  {}", MethodFormatter::new(method));

        if self.verbose {
            self.print_method_details(method);
        }
    }

    /// Print method code and other details in verbose mode
    fn print_method_details(&self, method: &Method) {
        if let Some(body) = &method.body {
            self.print_method_body(body);
        }

        if method.is_synthetic {
            println!("    Synthetic: true");
        }
        if method.is_deprecated {
            println!("    Deprecated: true");
        }

        self.print_method_exceptions(method);
    }

    /// Print method body information (code, stack info, etc.)
    fn print_method_body(&self, body: &MethodBody) {
        println!("    Code:");
        println!("      stack={}, locals={}", body.max_stack, body.max_locals);

        for (pc, instruction) in body.instructions.iter() {
            println!("      {pc}: {instruction}");
        }

        if !body.exception_table.is_empty() {
            self.print_exception_table(body);
        }
    }

    /// Print exception table for method body
    fn print_exception_table(&self, body: &MethodBody) {
        println!("      Exception table:");
        println!("         from    to  target type");
        for entry in &body.exception_table {
            let start_pc = *entry.covered_pc.start();
            let end_pc = *entry.covered_pc.end();
            let catch_type = match &entry.catch_type {
                Some(class_ref) => &class_ref.binary_name,
                None => "any",
            };
            println!("        {} {} {} {}", start_pc, end_pc, entry.handler_pc, catch_type);
        }
    }

    /// Print method exceptions information
    fn print_method_exceptions(&self, method: &Method) {
        if !method.exceptions.is_empty() {
            println!("    Exceptions:");
            for exception in &method.exceptions {
                println!("      {}", exception.binary_name);
            }
        }
    }
}
