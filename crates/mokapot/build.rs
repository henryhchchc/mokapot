//! Build script for the mokapot crate.

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
    thread,
};

use anyhow::{Context, ensure};

const SKIP_JAVA_TESTS: &str = "MOKAPOT_SKIP_JAVA_TESTS";
static OUT_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")));

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
    let num_shards = if env::var("CI").is_ok() {
        num_cpus.get() // Use all CPUs on CI
    } else {
        num_cpus.get().saturating_sub(2).max(1) // Reserve 2 CPUs for dev machine
    };

    let shards_filename = OUT_DIR.join("jdk_class_shards.rs");
    let test_case_shards = File::create(shards_filename).context("creating shards file")?;
    let mut w = BufWriter::new(test_case_shards);

    writeln!(&mut w, "jdk_classes_smoke_tests! {{")?;
    for shard in 0..num_shards {
        writeln!(&mut w, "jdk_classes_smoke_test_shard_{shard} = {shard};")?;
    }
    writeln!(&mut w, "}}")?;
    Ok(())
}

fn build_java_fixtures() -> anyhow::Result<()> {
    println!("cargo::rustc-cfg=java_fixture_tests");

    let output = OUT_DIR.join("mokapot");
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
