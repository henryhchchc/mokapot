#![allow(missing_docs, clippy::ignore_without_reason)]

use std::{
    collections::HashSet,
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use mokapot::{
    ir::{ControlTransfer, MokaIRMethod, path_condition::PathCondition},
    jvm::Class,
};

/// Declares one sharded test per bin, and derives `BIN_COUNT` from the list so
/// that the shard count and the tests cannot drift apart.
macro_rules! jdk_class_bins {
    (@id $bin:literal) => {
        $bin
    };
    ($($name:ident = $bin:literal;)*) => {
        const BIN_COUNT: u64 = [$(jdk_class_bins!(@id $bin)),*].len() as u64;

        $(
            #[test]
            #[ignore = "CI Only"]
            fn $name() {
                test_jdk_classes::<$bin>();
            }
        )*
    };
}

jdk_class_bins! {
    works_with_jdk_classes_bin_0 = 0;
    works_with_jdk_classes_bin_1 = 1;
    works_with_jdk_classes_bin_2 = 2;
    works_with_jdk_classes_bin_3 = 3;
    works_with_jdk_classes_bin_4 = 4;
    works_with_jdk_classes_bin_5 = 5;
    works_with_jdk_classes_bin_6 = 6;
    works_with_jdk_classes_bin_7 = 7;
}

fn test_jdk_classes<const BIN: u64>() {
    let extracted_modules_images = env::var("JDK_CLASSES").unwrap();
    let extracted_modules_images = PathBuf::from(extracted_modules_images);
    let class_files: Vec<_> = walkdir::WalkDir::new(&extracted_modules_images)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|it| it.path().extension().is_some_and(|it| it == "class"))
        .map(walkdir::DirEntry::into_path)
        .collect();

    assert!(
        !class_files.is_empty(),
        "There is no class file in '{}'.",
        extracted_modules_images.display()
    );

    // Sequential on purpose: the analysis is allocation-heavy, and under
    // `-Cinstrument-coverage` parallel workers contend on the allocator and
    // coverage counters, which dominates the runtime (~7x slower on 10 threads
    // than on one).
    for class_file in class_files.into_iter().filter(|it| {
        let mut hasher = DefaultHasher::new();
        it.hash(&mut hasher);
        let hash = hasher.finish();
        hash % BIN_COUNT == BIN
    }) {
        let reader = fs::File::open(&class_file).unwrap();
        let mut buf_reader = std::io::BufReader::new(reader);
        let class = Class::from_reader(&mut buf_reader);
        match class {
            Ok(c) => test_a_class(c),
            Err(e) => {
                panic!("Failed to parse {}: {e}", class_file.display());
            }
        }
    }
}

fn test_a_class(class: Class) {
    class
        .methods
        .iter()
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
            let ir_method = MokaIRMethod::from_method(it).unwrap_or_else(|e| {
                panic!("Failed to build {}: {}", it.name, e);
            });
            let mut pending = vec![ir_method.entry_block()];
            let mut visited = HashSet::new();
            let mut variable_count = 0;
            while let Some(block) = pending.pop() {
                if !visited.insert(block) {
                    continue;
                }
                let terminator = &ir_method
                    .block(block)
                    .expect("successor blocks belong to the method")
                    .terminator;
                for successor in terminator.successors() {
                    if let Some(target) = successor.block_target() {
                        pending.push(target);
                    }
                    if let Some(ControlTransfer::Conditional(guard)) = successor.transfer() {
                        variable_count += guard.predicate_count();
                    }
                }
            }
            // Set a limit here due to high resource consumption.
            // [TODO] optimized later.
            let variable_count_limit = if env::var("CI").is_ok() { 8 } else { 16 };
            if variable_count <= variable_count_limit {
                let _ = PathCondition::analyze(&ir_method);
            }
        });

    let mut class_bytes = Vec::new();
    class.to_writer(&mut class_bytes).unwrap();
    Class::from_reader(&mut class_bytes.as_slice()).unwrap();
}
