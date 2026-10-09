//! Reproducible JDK benchmark selection.

use std::path::Path;

#[path = "../benches/support/jdk_corpus.rs"]
#[expect(
    dead_code,
    reason = "selection tests do not exercise the corpus loader"
)]
mod jdk_corpus;
use jdk_corpus::{Shard, relative_name};

#[test]
fn selection_is_stable_across_extraction_roots() {
    let first = relative_name(
        Path::new("one"),
        Path::new("one/java.base/java/lang/Object.class"),
    )
    .unwrap();
    let second = relative_name(
        Path::new("two"),
        Path::new("two/java.base/java/lang/Object.class"),
    )
    .unwrap();
    assert_eq!(first, "java.base/java/lang/Object.class");
    assert_eq!(first, second);
    // FNV-1a's published test vector fixes the algorithm independently of Rust's hasher.
    assert!(
        Shard::new(Some("11"), Some("16"))
            .unwrap()
            .contains("hello")
    );
    for index in 0..16 {
        let shard = Shard::new(Some(&index.to_string()), Some("16")).unwrap();
        assert_eq!(shard.contains(&first), shard.contains(&second));
    }
}

#[test]
fn shards_partition_the_corpus_and_one_shard_selects_all() {
    let full = Shard::new(Some("0"), Some("1")).unwrap();
    let shards: Vec<_> = (0..16)
        .map(|index| Shard::new(Some(&index.to_string()), Some("16")).unwrap())
        .collect();
    for index in 0..1024 {
        let name = format!("java.base/example/Class{index}.class");
        assert!(full.contains(&name));
        assert_eq!(
            shards.iter().filter(|shard| shard.contains(&name)).count(),
            1
        );
    }
}
