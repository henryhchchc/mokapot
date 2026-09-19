//! Builds and prints MokaIR for each concrete method in a class file.

use std::{env, fs::File, io::BufReader, path::PathBuf};

use mokapot::{
    ir::{InstructionLocation, MokaIRMethod},
    jvm::Class,
};

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

        for (block_id, block) in ir.blocks() {
            println!("{block_id}:");
            for (index, phi) in block.phis.iter().enumerate() {
                let loc = InstructionLocation::Phi {
                    block: block_id,
                    index,
                };
                print!("  {:?}: {} = phi", loc, phi.value);
                for input in &phi.inputs {
                    print!(" [{}: {}]", input.predecessor, input.value);
                }
                println!();
            }
            for (index, operation) in block.operations.iter().enumerate() {
                let loc = InstructionLocation::Operation {
                    block: block_id,
                    index,
                };
                println!("  {:?}: {operation}", loc);
            }

            let terminator = &block.terminator;
            let loc = InstructionLocation::Terminator { block: block_id };
            println!("  {:?}: {terminator}", loc);
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
