//! Builds and prints MokaIR for each concrete method in a class file.

use std::{env, fs::File, io::BufReader, path::PathBuf};

use mokapot::{ir::MokaIRMethod, jvm::Class};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: moka_ir <class-file>")?;
    let mut reader = BufReader::new(File::open(path)?);
    let class = Class::from_reader(&mut reader)?;

    for method in &class.methods {
        if method.body.is_none() {
            continue;
        }

        let ir = MokaIRMethod::from_method(method)?;
        println!("{}{}:", ir.name(), ir.descriptor());

        for block in ir.blocks() {
            println!("{}:", block.id());
            for phi in block.phis() {
                print!("  {}: {} = phi", phi.id(), phi.value());
                for input in phi.inputs() {
                    print!(" [{}: {}]", input.predecessor(), input.value());
                }
                println!();
            }
            for operation in block.operations() {
                println!("  {}: {operation}", operation.id());
            }

            let terminator = block.terminator();
            println!("  {}: {terminator}", terminator.id());
            for successor in terminator.successors() {
                println!(
                    "    {} -> {} ({:?})",
                    successor.id(),
                    successor.target(),
                    successor.transfer(),
                );
            }
        }
    }

    Ok(())
}
