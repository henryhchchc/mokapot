//! Build script for the mokapot crate.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const SKIP_JAVA_TESTS: &str = "MOKAPOT_SKIP_JAVA_TESTS";

fn main() {
    println!("cargo::rustc-check-cfg=cfg(java_fixture_tests)");
    println!("cargo::rerun-if-env-changed={SKIP_JAVA_TESTS}");
    println!("cargo::rerun-if-changed=test_data");

    match env::var(SKIP_JAVA_TESTS).as_deref() {
        Ok("1") => return,
        Err(env::VarError::NotPresent) => {}
        _ => panic!("{SKIP_JAVA_TESTS} must be unset or set to 1"),
    }
    println!("cargo::rustc-cfg=java_fixture_tests");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("mokapot");
    fs::create_dir_all(&output).expect("cannot create Java fixture output directory");
    let error_path = output.join("fixture_error.txt");
    if error_path.exists() {
        fs::remove_file(&error_path).expect("cannot remove previous Java fixture error");
    }
    if let Err(error) = compile_java_test_data(&output) {
        fs::write(&error_path, &error).expect("cannot record Java fixture error");
        println!("cargo::warning={error}");
    }
}

fn compile_java_test_data(output: &Path) -> Result<(), String> {
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join("mokapot");
    let pattern = format!("{}/**/*.java", source_dir.display());
    let mut sources = glob::glob(&pattern)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    sources.sort();
    if sources.is_empty() {
        return Err("No Java fixture sources found".to_owned());
    }

    let classes = output.join("java_classes");
    if classes.exists() {
        fs::remove_dir_all(&classes).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&classes).map_err(|error| error.to_string())?;

    let javac = Command::new("javac")
        .current_dir(&source_dir)
        .arg("-g")
        .arg("-d")
        .arg(&classes)
        .args(&sources)
        .output()
        .map_err(|error| format!("Cannot run javac for Java test fixtures: {error}"))?;
    if !javac.status.success() {
        return Err(format!(
            "Cannot compile Java test fixtures: {}",
            String::from_utf8_lossy(&javac.stderr)
        ));
    }

    let jar = output.join("test_classes.jar");
    let result = Command::new("jar")
        .args(["--create", "--file"])
        .arg(&jar)
        .arg("-C")
        .arg(&classes)
        .arg(".")
        .output()
        .map_err(|error| format!("Cannot run jar for Java test fixtures: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "Cannot package Java test fixtures: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}
