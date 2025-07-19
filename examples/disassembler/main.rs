use std::fs::File;
use std::io::{BufReader, Result as IoResult};
use std::path::PathBuf;

use clap::Parser;
use mokapot::jvm::{self, Class};
use thiserror::Error;

/// Command line arguments
#[derive(Parser)]
#[command(name = "javap")]
#[command(about = "Disassembles class files", long_about = None)]
struct Args {
    /// Class file to process
    #[arg(required = true)]
    class_files: Vec<PathBuf>,

    /// Output format - plain or verbose
    #[arg(
        short = 'v',
        long = "verbose",
        help = "Display verbose output with additional details"
    )]
    verbose: bool,
}

/// Custom error type for disassembler operations
#[derive(Debug, Error)]
enum DisassemblerError {
    /// Error when reading files
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Error when parsing class files
    #[error("Class parse error: {0}")]
    ClassParseError(#[from] jvm::bytecode::ParseError),
}

fn main() -> IoResult<()> {
    let args = Args::parse();

    process_class_files(&args.class_files, args.verbose)
}

/// Formatters for Java class components
mod formatters;

/// Printer for class information
mod printer;

/// Process all specified class files
fn process_class_files(class_files: &[PathBuf], verbose: bool) -> IoResult<()> {
    let multiple_files = class_files.len() > 1;

    for (idx, class_path) in class_files.iter().enumerate() {
        if multiple_files {
            println!("Classfile {}", class_path.display());
            println!();
        }

        match parse_class_file(class_path) {
            Ok(class) => {
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
fn parse_class_file(path: &PathBuf) -> Result<Class, DisassemblerError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let class = Class::from_reader(&mut reader)?;
    Ok(class)
}
