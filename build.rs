use std::{env, path::PathBuf, process::Command};

fn main() {
    compile_java_test_data();
}

fn compile_java_test_data() {
    if Command::new("javac").spawn().is_ok() {
        compile_java_files("mokapot");
        compile_java_files("openjdk");
        println!("cargo:rustc-cfg=test_with_jdk");
    } else {
        println!("cargo:warning=Can not find javac, test compilation will fail");
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
            .expect("Test folder is not named with vaild UTF-8")
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
                .expect("Java source file is not named with vaild UTF-8")
                .to_owned()
        }))
        .output()
        .expect("Fail to spawn javac");

    if !status.status.success() {
        println!(
            "cargo:warning=Failed to compile java files: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
}
