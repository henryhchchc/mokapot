#![cfg(integration_test)]

use mokapot::{ir::MokaIRMethodExt, jvm::Class};
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
        let mut buf_reader = std::io::BufReader::new(reader);
        let class = Class::read_from(&mut buf_reader);
        match class {
            Ok(c) => test_a_class(c),
            Err(e) => {
                panic!("Failed to parse {:?}: {}", class_file, e);
            }
        }
    });
}

fn test_a_class(class: Class) {
    class
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
            let _ir_method = it.brew().unwrap_or_else(|e| {
                panic!("Failed to brew {}: {}", it.name, e);
            });
        });

    let mut class_bytes = Vec::new();
    class.write_to(&mut class_bytes).unwrap();
    Class::read_from(&mut class_bytes.as_slice()).unwrap();
}
