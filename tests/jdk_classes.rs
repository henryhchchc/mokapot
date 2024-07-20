#![cfg(integration_test)]

use mokapot::{
    analysis::{fixed_point::Analyzer, path_condition},
    ir::MokaIRMethodExt,
    jvm::Class,
};
use rayon::prelude::*;
use std::{env, fs, path::PathBuf};

#[test]
#[ignore = "CI Only"]
fn works_with_jdk_classes() {
    let extracted_modules_images = env::var("JDK_CLASSES").unwrap();
    let extracted_modules_images = PathBuf::from(extracted_modules_images);
    let class_files: Vec<_> = walkdir::WalkDir::new(extracted_modules_images)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|it| it.path().extension().is_some_and(|it| it == "class"))
        .map(|it| it.into_path())
        .collect();

    class_files.into_par_iter().for_each(|class_file| {
        let reader = fs::File::open(&class_file).unwrap();
        let buf_reader = std::io::BufReader::new(reader);
        let class = Class::from_reader(buf_reader);
        match class {
            Ok(c) => c
                .methods
                .par_iter()
                .filter(|it| {
                    it.body
                        .as_ref()
                        // Skip large method to speed up the test
                        .is_some_and(|it| it.instructions.len() < 512)
                })
                .for_each(|it| {
                    it.body
                        .as_ref()
                        .unwrap()
                        .instructions
                        .iter()
                        .for_each(|(_pc, insn)| {
                            let _ = insn.name();
                        });
                    let ir_method = it.brew().unwrap_or_else(|e| {
                        panic!("Failed to brew {}: {}", it.name, e);
                    });
                    let mut path_cond = path_condition::Analyzer::new(&ir_method);
                    let _pc = path_cond.analyze().unwrap_or_else(|e| {
                        panic!(
                            "Failed to analyze path conditions for {}: {}",
                            ir_method.name, e
                        );
                    });
                }),
            Err(e) => {
                panic!("Failed to parse {:?}: {}", class_file, e);
            }
        }
    });
}
