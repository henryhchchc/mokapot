#![allow(missing_docs, clippy::ignore_without_reason)]

use std::{
    collections::HashSet,
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    io::BufReader,
    path::{Path, PathBuf},
};

use mokapot::{
    ir::{ControlTransfer, MokaIRMethod, path_condition::PathCondition},
    jvm::Class,
    types::Descriptor,
};

/// Declares one sharded test per bin, and derives `BIN_COUNT` from the list so
/// that the shard count and the tests cannot drift apart.
macro_rules! jdk_class_bins {
    (@count $bin:literal) => {
        1
    };
    ($($name:ident = $bin:literal;)*) => {
        const BIN_COUNT: u64 = [$(jdk_class_bins!(@count $bin)),*].len() as u64;

        $(
            #[test]
            #[ignore = "requires an extracted JDK image"]
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

/// Whether `path` falls in the shard `BIN` of `BIN_COUNT`.
///
/// Each shard process walks the whole corpus and keeps its own slice, so the
/// shards together cover it exactly once. That relies on the hasher being
/// seeded with fixed keys: `RandomState` would scatter the corpus differently
/// per process, leaving gaps and overlaps.
fn in_shard<const BIN: u64>(path: &Path) -> bool {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish() % BIN_COUNT == BIN
}

fn test_jdk_classes<const BIN: u64>() {
    let root = env::var("JDK_CLASSES").expect("JDK_CLASSES must point at the extracted JDK image");
    let root = PathBuf::from(root);
    let mut shard = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|it| it.path().extension().is_some_and(|ext| ext == "class"))
        .map(walkdir::DirEntry::into_path)
        .filter(|it| in_shard::<BIN>(it))
        .peekable();
    assert!(
        shard.peek().is_some(),
        "Shard {BIN} of {BIN_COUNT} has no class file under '{}'.",
        root.display()
    );

    // The cost of path-condition analysis grows with the predicate count, so
    // bound it to keep the instrumented CI run in budget.
    // [TODO] optimize and lift the bound.
    let variable_count_limit = if env::var("CI").is_ok() { 8 } else { 16 };

    // Sequential on purpose: the analysis is allocation-heavy, and under
    // `-Cinstrument-coverage` parallel workers contend on the allocator and
    // coverage counters, which dominates the runtime (~7x slower on 10 threads
    // than on one). Shards get their concurrency from separate processes.
    for class_file in shard {
        let mut reader = BufReader::new(fs::File::open(&class_file).unwrap());
        match Class::from_reader(&mut reader) {
            Ok(class) => test_a_class(class, variable_count_limit),
            Err(e) => panic!("Failed to parse {}: {e}", class_file.display()),
        }
    }
}

/// Counts the predicates guarding the successors reachable from the entry
/// block, which is what the path-condition analysis cost grows with.
fn reachable_predicate_count(ir_method: &MokaIRMethod) -> usize {
    let mut pending = vec![ir_method.entry.block];
    let mut visited = HashSet::new();
    let mut count = 0;
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
                count += guard.predicate_count();
            }
        }
    }
    count
}

fn test_a_class(class: Class, variable_count_limit: usize) {
    for method in &class.methods {
        let Some(body) = method.body.as_ref() else {
            continue;
        };

        // Naming every instruction exercises the decoder; the names themselves
        // are not asserted on.
        for (_pc, insn) in body.instructions.iter() {
            let _ = insn.name();
        }

        let ir_method = MokaIRMethod::from_method(method).unwrap_or_else(|e| {
            panic!(
                "Failed to build {}::{}{}: {e}",
                class.binary_name,
                method.name,
                method.descriptor.descriptor()
            );
        });

        if reachable_predicate_count(&ir_method) <= variable_count_limit {
            let _ = PathCondition::analyze(&ir_method);
        }
    }

    // Round-trip the class to exercise generation as well as parsing.
    let mut class_bytes = Vec::new();
    class.to_writer(&mut class_bytes).unwrap();
    Class::from_reader(&mut class_bytes.as_slice()).unwrap();
}
