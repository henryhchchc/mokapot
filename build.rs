use std::{env, path::PathBuf, process::Command};

fn main() {
    compile_java_test_data();
}

const INTEGRATION_TEST: &str = "INTEGRATION_TEST";

fn compile_java_test_data() {
    println!("cargo::rustc-check-cfg=cfg(integration_test)");
    println!("cargo::rerun-if-env-changed={INTEGRATION_TEST}");
    if env::var(INTEGRATION_TEST).is_ok() {
        println!("cargo::rustc-cfg=integration_test");
    }
    if Command::new("javac").spawn().is_ok() {
        compile_java_files("mokapot");
        println!("cargo::rerun-if-changed=test_data");
    } else {
        println!("cargo::warning=Can not find javac, test compilation will fail");
    }
}

fn compile_java_files(path: &str) {
    let build_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let test_data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(path);
    let glob_pattern = format!(
        "{}/**/*.java",
        test_data_path
            .to_str()
            .expect("Test folder is not named with valid UTF-8")
    );
    let java_source_files: Vec<_> = glob::glob(&glob_pattern)
        .expect("The glob pattern is invalid.")
        .filter_map(Result::ok)
        .collect();

    let status = Command::new("javac")
        .current_dir(test_data_path)
        .arg("-g")
        .arg("-d")
        .arg(build_path.join(path).join("java_classes"))
        .args(java_source_files.into_iter().map(|it| {
            it.to_str()
                .expect("Java source file is not named with valid UTF-8")
                .to_owned()
        }))
        .output()
        .expect("Fail to spawn javac");

    if !status.status.success() {
        println!(
            "cargo::warning=Failed to compile java files: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
}
