//! Class file printer implementation
//!
//! This module provides the main printer for Java class files.
//! It handles the detailed display of class structure, fields,
//! methods, and bytecode instructions in a format similar to
//! what the `javap` tool produces.

use mokapot::jvm::{Class, Field, Method};
use mokapot::jvm::code::MethodBody;

use crate::formatters::{ClassFormatter, FieldFormatter, MethodFormatter};

/// Structure for printing class information with consistent formatting
///
/// The ClassPrinter is responsible for displaying all aspects of a Java class file,
/// including class declaration, fields, methods, and bytecode instructions.
/// It supports both basic and verbose output modes, with the latter showing
/// additional details like bytecode instructions and constant values.
pub struct ClassPrinter<'a> {
    /// Reference to the class being printed
    class: &'a Class,
    
    /// Whether to print verbose details including bytecode instructions
    verbose: bool,
}

impl<'a> ClassPrinter<'a> {
    /// Creates a new class printer for the given class
    ///
    /// # Arguments
    ///
    /// * `class` - Reference to the Class to be printed
    /// * `verbose` - Whether to include detailed information in the output
    pub fn new(class: &'a Class, verbose: bool) -> Self {
        Self { class, verbose }
    }

    /// Prints all class information in a structured format
    ///
    /// This is the main entry point for displaying a class. The output follows
    /// a format similar to Java source code:
    /// 1. Class declaration with access modifiers
    /// 2. Fields with their types and modifiers
    /// 3. Methods with their signatures and bodies (if verbose)
    ///
    /// In verbose mode, additional details like bytecode instructions
    /// and constant values are included.
    pub fn print(&self) {
        // Print class header (name, modifiers, inheritance)
        self.print_header();

        // In verbose mode, print additional class details
        if self.verbose {
            self.print_details();
        }

        println!();
        println!("{} {{", self.class.binary_name);

        // Print fields section
        for field in &self.class.fields {
            self.print_field(field);
        }

        println!();

        // Print methods section
        for method in &self.class.methods {
            self.print_method(method);
        }

        println!("}}");
    }

    /// Prints the class header information
    ///
    /// Displays the class declaration including:
    /// - Access modifiers (public, abstract, etc.)
    /// - Class name
    /// - Parent class (extends)
    /// - Implemented interfaces
    ///
    /// In verbose mode, also shows the source file name (if available).
    fn print_header(&self) {
        // In verbose mode, show the source file information
        if self.verbose {
            println!("Compiled from \"Unknown source\"");
        }

        // Format access flags like public, final, abstract, etc.
        let formatter = ClassFormatter::new(self.class);
        let access_flags = formatter.format_access_flags();

        // Print class declaration with or without access flags
        if access_flags.is_empty() {
            println!("class {}", self.class.binary_name);
        } else {
            println!("{} class {}", access_flags, self.class.binary_name);
        }

        // Print parent class if present (all classes except Object have a parent)
        if let Some(super_class) = &self.class.super_class {
            println!("  extends {}", super_class.binary_name);
        }

        // Print implemented interfaces if any
        if !self.class.interfaces.is_empty() {
            print!("  implements ");
            // Use peekable iterator to handle comma placement correctly
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

    /// Prints detailed class information (for verbose mode only)
    ///
    /// Displays technical details about the class file:
    /// - JVM class file version (major.minor)
    /// - Raw access flags as hexadecimal value
    ///
    /// This information is useful for understanding the low-level
    /// representation of the class file.
    fn print_details(&self) {
        println!("  minor version: {}", self.class.version.minor());
        println!("  major version: {}", self.class.version.major());
        println!("  flags: {:#06x}", self.class.access_flags.bits());
    }

    /// Prints information about a class field
    ///
    /// Displays a single field declaration including:
    /// - Access modifiers (public, static, etc.)
    /// - Field type
    /// - Field name
    /// - Constant value (if present)
    ///
    /// In verbose mode, also shows additional attributes like
    /// whether the field is synthetic or deprecated.
    ///
    /// # Arguments
    ///
    /// * `field` - Reference to the Field to print
    fn print_field(&self, field: &Field) {
        // Use the field formatter to display the basic field declaration
        println!("  {}", FieldFormatter::new(field));

        // In verbose mode, print additional field metadata
        if self.verbose {
            self.print_field_details(field);
        }
    }

    /// Prints additional field details in verbose mode
    ///
    /// Displays metadata attributes of a field that aren't part of
    /// its normal declaration:
    /// - Synthetic flag (compiler-generated fields)
    /// - Deprecated flag
    /// - Annotations (both runtime visible and invisible)
    ///
    /// # Arguments
    ///
    /// * `field` - Reference to the Field whose details should be printed
    fn print_field_details(&self, field: &Field) {
        // Show if the field was generated by the compiler (not in source code)
        if field.is_synthetic {
            println!("    Synthetic: true");
        }
        // Show if the field is marked as deprecated (with @Deprecated annotation)
        if field.is_deprecated {
            println!("    Deprecated: true");
        }
        // Show count of runtime visible annotations (available at runtime via reflection)
        if !field.runtime_visible_annotations.is_empty() {
            println!("    Runtime visible annotations: {}", field.runtime_visible_annotations.len());
        }
        // Show count of runtime invisible annotations (not available at runtime)
        if !field.runtime_invisible_annotations.is_empty() {
            println!("    Runtime invisible annotations: {}", field.runtime_invisible_annotations.len());
        }
    }

    /// Prints information about a class method
    ///
    /// Displays a single method declaration including:
    /// - Access modifiers (public, static, synchronized, etc.)
    /// - Return type
    /// - Method name
    /// - Parameter list
    ///
    /// In verbose mode, also shows the method body with bytecode instructions.
    ///
    /// # Arguments
    ///
    /// * `method` - Reference to the Method to print
    fn print_method(&self, method: &Method) {
        // Use the method formatter to display the basic method signature
        println!("  {}", MethodFormatter::new(method));

        // In verbose mode, print additional method details and bytecode
        if self.verbose {
            self.print_method_details(method);
        }
    }

    /// Prints method code and other details in verbose mode
    ///
    /// Displays the detailed implementation of a method, including:
    /// - Bytecode instructions (if method has a body)
    /// - Metadata flags like synthetic or deprecated
    /// - Declared exceptions (throws clause)
    ///
    /// Abstract and native methods won't have a method body.
    ///
    /// # Arguments
    ///
    /// * `method` - Reference to the Method whose details should be printed
    fn print_method_details(&self, method: &Method) {
        // Print bytecode if the method has an implementation
        // (abstract and native methods won't have a body)
        if let Some(body) = &method.body {
            self.print_method_body(body);
        }

        // Show if the method was generated by the compiler (not in source code)
        if method.is_synthetic {
            println!("    Synthetic: true");
        }
        // Show if the method is marked as deprecated
        if method.is_deprecated {
            println!("    Deprecated: true");
        }

        // Print the exceptions that this method can throw
        self.print_method_exceptions(method);
    }

    /// Prints method body information (code, stack info, etc.)
    ///
    /// Displays the bytecode instructions and execution environment of a method:
    /// - Stack size and local variable count
    /// - Bytecode instructions with program counter offsets
    /// - Exception table (try-catch blocks in bytecode)
    ///
    /// This is the most detailed part of the disassembly, showing the
    /// actual JVM instructions that will be executed.
    ///
    /// # Arguments
    ///
    /// * `body` - Reference to the MethodBody containing the bytecode
    fn print_method_body(&self, body: &MethodBody) {
        println!("    Code:");
        println!("      stack={}, locals={}", body.max_stack, body.max_locals);

        // Print each bytecode instruction with its program counter offset
        for (pc, instruction) in body.instructions.iter() {
            println!("      {pc}: {instruction}");
        }

        // If the method has exception handlers (try-catch blocks), print them
        if !body.exception_table.is_empty() {
            self.print_exception_table(body);
        }
    }

    /// Prints exception table for method body
    ///
    /// Displays the exception handlers (try-catch blocks) in a method:
    /// - Start and end offsets of the protected region (try block)
    /// - Handler offset where execution jumps when exception occurs
    /// - Exception type that this handler catches (or "any" for finally blocks)
    ///
    /// The exception table maps directly to try-catch-finally blocks in Java code,
    /// but represented as instruction ranges in bytecode.
    ///
    /// # Arguments
    ///
    /// * `body` - Reference to the MethodBody containing the exception table
    fn print_exception_table(&self, body: &MethodBody) {
        println!("      Exception table:");
        println!("         from    to  target type");
        for entry in &body.exception_table {
            // Get the start and end program counters of the protected region
            let start_pc = *entry.covered_pc.start();
            let end_pc = *entry.covered_pc.end();
            
            // Get the exception type, or "any" for finally blocks (which catch all exceptions)
            let catch_type = match &entry.catch_type {
                Some(class_ref) => &class_ref.binary_name,
                None => "any", // "any" means a finally block or catch-all handler
            };
            
            println!("        {} {} {} {}", start_pc, end_pc, entry.handler_pc, catch_type);
        }
    }

    /// Prints method exceptions information
    ///
    /// Displays the declared exceptions that a method can throw
    /// (corresponding to the throws clause in Java source code).
    /// These are the checked exceptions that callers must handle.
    ///
    /// # Arguments
    ///
    /// * `method` - Reference to the Method whose exceptions should be printed
    fn print_method_exceptions(&self, method: &Method) {
        if !method.exceptions.is_empty() {
            println!("    Exceptions:");
            for exception in &method.exceptions {
                println!("      {}", exception.binary_name);
            }
        }
    }
}
