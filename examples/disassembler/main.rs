//! Java class file disassembler example
//!
//! This example demonstrates how to use the mokapot library to parse and 
//! display the contents of Java class files. It provides functionality
//! similar to the Java `javap` tool, showing class structure, fields,
//! methods, and bytecode instructions.

use std::fs::File;
use std::io::{BufReader, Result as IoResult};
use std::path::PathBuf;

use clap::Parser;
use mokapot::jvm::{self, Class};
use thiserror::Error;

/// Command line arguments for the disassembler
/// 
/// This struct mirrors the `javap` tool's command-line interface,
/// allowing users to specify class files and output format options.
#[derive(Parser)]
#[command(name = "javap")]
#[command(about = "Disassembles class files", long_about = None)]
struct Args {
    /// Class file(s) to process - accepts multiple files
    #[arg(required = true)]
    class_files: Vec<PathBuf>,

    /// Output format - plain or verbose
    /// When enabled, shows additional information like bytecode instructions,
    /// constant values, stack sizes, and exception tables
    #[arg(
        short = 'v',
        long = "verbose",
        help = "Display verbose output with additional details"
    )]
    verbose: bool,
}

/// Custom error type for disassembler operations
/// 
/// Uses thiserror to provide clean error handling with automatic
/// conversion from common error types and formatted error messages.
#[derive(Debug, Error)]
enum DisassemblerError {
    /// Error when reading files - occurs when files can't be opened or read
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Error when parsing class files - occurs when file content is invalid
    /// or doesn't conform to the Java class file format specification
    #[error("Class parse error: {0}")]
    ClassParseError(#[from] jvm::bytecode::ParseError),
}

/// Entry point for the disassembler
/// 
/// Parses command-line arguments and processes the specified class files.
/// Returns an I/O Result to properly handle potential errors.
fn main() -> IoResult<()> {
    // Parse command line arguments using clap
    let args = Args::parse();

    // Process all specified class files with the given verbosity setting
    process_class_files(&args.class_files, args.verbose)
}

/// Formatters for Java class components
/// 
/// This module contains types that implement Display for class components,
/// converting JVM structures into human-readable text representations.
/// It handles formatting of class access flags, field declarations,
/// and method signatures.
mod formatters;

/// Printer for class information
/// 
/// This module contains the ClassPrinter that orchestrates the display of
/// all class information, including class structure, fields, methods,
/// and bytecode instructions when in verbose mode.
mod printer;

/// Process all specified class files
/// 
/// Iterates through each class file path, attempts to parse it, and prints
/// its contents. When processing multiple files, adds headers and separators.
///
/// # Arguments
///
/// * `class_files` - Slice of paths to the class files to process
/// * `verbose` - Whether to display detailed information including bytecode
///
/// # Returns
///
/// An IoResult indicating success or any I/O errors that occurred
fn process_class_files(class_files: &[PathBuf], verbose: bool) -> IoResult<()> {
    // Flag to determine if we need file headers and separators
    let multiple_files = class_files.len() > 1;

    for (idx, class_path) in class_files.iter().enumerate() {
        // For multiple files, add a header with the file name
        if multiple_files {
            println!("Classfile {}", class_path.display());
            println!();
        }

        // Parse and print the class file, or display any errors
        match parse_class_file(class_path) {
            Ok(class) => {
                // Create a printer configured for the desired verbosity
                let printer = printer::ClassPrinter::new(&class, verbose);
                printer.print();
            }
            Err(e) => {
                eprintln!("Error parsing class file {}: {}", class_path.display(), e);
            }
        }

        // Add separator between files except after the last one
        if multiple_files && idx < class_files.len() - 1 {
            println!("\n{}", "-".repeat(80));
        }
    }

    Ok(())
}

/// Parse a class file into a Class structure
///
/// Opens and reads the class file at the specified path, then uses mokapot
/// to parse it into a Class object that can be analyzed and displayed.
///
/// # Arguments
///
/// * `path` - Path to the class file to parse
///
/// # Returns
///
/// A Result containing either the parsed Class or a DisassemblerError
/// 
/// # Error Handling
///
/// This function will automatically convert I/O errors or parsing errors
/// into the appropriate DisassemblerError variant through the ? operator
/// and the From trait implementations on DisassemblerError.
fn parse_class_file(path: &PathBuf) -> Result<Class, DisassemblerError> {
    // Open the file (may fail with IoError)
    let file = File::open(path)?;
    
    // Create a buffered reader for efficient reading
    let mut reader = BufReader::new(file);
    
    // Parse the class file (may fail with ParseError)
    let class = Class::from_reader(&mut reader)?;
    
    Ok(class)
}
