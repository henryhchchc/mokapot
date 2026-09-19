//! Builds and prints MokaIR for each concrete method in a class file.

use std::{collections::HashSet, env, fs::File, io::BufReader, path::PathBuf};

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

        let mut pending = vec![ir.entry_block()];
        let mut visited = HashSet::new();
        while let Some(block) = pending.pop() {
            if !visited.insert(block) {
                continue;
            }
            let bb = ir
                .block(block)
                .expect("a successor must belong to its method");
            println!("{block}:");
            for (index, parameter) in bb.parameters.iter().enumerate() {
                let loc = InstructionLocation::BlockParameter { block, index };
                println!("  {:?}: parameter {}", loc, parameter.value);
            }
            for (index, operation) in bb.operations.iter().enumerate() {
                let loc = InstructionLocation::Operation { block, index };
                println!("  {:?}: {operation}", loc);
            }

            let loc = InstructionLocation::Terminator { block };
            println!("  {:?}: {}", loc, bb.terminator);
            for successor in bb.terminator.successors() {
                pending.extend(successor.block_target());
                println!(
                    "    {} -> {:?} {:?} ({:?})",
                    successor.id(),
                    successor.block_target(),
                    successor.arguments(),
                    successor.transfer(),
                );
            }
        }
    }

    Ok(())
}
