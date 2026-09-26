#![allow(
    dead_code,
    reason = "each integration target uses different fixture helpers"
)]

use std::{fs, path::PathBuf};

fn output_dir() -> PathBuf {
    let output = PathBuf::from(env!("OUT_DIR")).join("mokapot");
    let error = output.join("fixture_error.txt");
    if error.exists() {
        let message = fs::read_to_string(error).expect("cannot read Java fixture build error");
        panic!("Java test fixtures are unavailable: {message}");
    }
    output
}

pub fn classes_dir() -> PathBuf {
    let path = output_dir().join("java_classes");
    assert!(
        path.is_dir(),
        "Java fixture classes are missing: {}",
        path.display()
    );
    path
}

pub fn class_bytes(name: &str) -> Vec<u8> {
    let path = classes_dir().join(format!("{name}.class"));
    fs::read(&path).unwrap_or_else(|error| panic!("Cannot read {}: {error}", path.display()))
}

pub fn jar_path() -> PathBuf {
    let path = output_dir().join("test_classes.jar");
    assert!(
        path.is_file(),
        "Java fixture JAR is missing: {}",
        path.display()
    );
    path
}
