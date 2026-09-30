//! Build script for the mokapot crate.

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
};

use anyhow::{Context, ensure};

const SKIP_JAVA_TESTS: &str = "MOKAPOT_SKIP_JAVA_TESTS";

fn main() -> anyhow::Result<()> {
    println!("cargo::rustc-check-cfg=cfg(java_fixture_tests)");
    println!("cargo::rerun-if-env-changed={SKIP_JAVA_TESTS}");
    println!("cargo::rerun-if-changed=test_data");

    generate_jdk_classes_shards()?;

    match env::var(SKIP_JAVA_TESTS).as_deref() {
        Ok("1") => {}
        Err(env::VarError::NotPresent) => build_java_fixtures()?,
        _ => panic!("{SKIP_JAVA_TESTS} must be unset or set to 1"),
    }

    Ok(())
}

fn generate_jdk_classes_shards() -> anyhow::Result<()> {
    let num_cpus = thread::available_parallelism().context("getting number of CPUs")?;
    let num_shards = num_cpus.get().saturating_sub(2).max(1);

    let shards_fragment_fn = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"))
        .join("jdk_class_bins.rs");
    let shards_fragment = File::create(shards_fragment_fn).context("creating jdk_class_bins.rs")?;
    let mut writer = BufWriter::new(shards_fragment);
    let f = writer.by_ref();

    writeln!(f, "jdk_class_bins! {{")?;
    for bin in 0..num_shards {
        writeln!(f, "works_with_jdk_classes_bin_{bin} = {bin};")?;
    }
    writeln!(f, "}}")?;
    Ok(())
}

fn build_java_fixtures() -> anyhow::Result<()> {
    println!("cargo::rustc-cfg=java_fixture_tests");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("mokapot");
    fs::create_dir_all(&output).context("creating Java fixture output directory")?;
    let error_path = output.join("fixture_error.txt");
    if error_path.exists() {
        fs::remove_file(&error_path).context("removing previous Java fixture error")?;
    }
    if let Err(error) = compile_java_test_data(&output) {
        println!("cargo::warning={error}");
    }
    Ok(())
}

fn compile_java_test_data(output: &Path) -> anyhow::Result<()> {
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join("mokapot");
    let pattern = format!("{}/**/*.java", source_dir.display());
    let mut sources = glob::glob(&pattern)
        .context("performing glob")?
        .collect::<Result<Vec<_>, _>>()?;
    sources.sort();
    ensure!(!sources.is_empty(), "No Java fixture sources found");

    let classes = output.join("java_classes");
    if classes.exists() {
        fs::remove_dir_all(&classes).context("Cleaning up classes")?;
    }
    fs::create_dir_all(&classes).context("Creating classes directory")?;

    let javac = Command::new("javac")
        .current_dir(&source_dir)
        .arg("-g")
        .arg("-d")
        .arg(&classes)
        .args(&sources)
        .output()
        .context("Compiling java test fixtures")?;
    ensure!(
        javac.status.success(),
        "Cannot compile Java test fixtures: {}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let jar = output.join("test_classes.jar");
    let result = Command::new("jar")
        .args(["--create", "--file"])
        .arg(&jar)
        .arg("-C")
        .arg(&classes)
        .arg(".")
        .output()
        .context("Packaging Java test fixtures")?;
    ensure!(
        result.status.success(),
        "Cannot package Java test fixtures: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}
