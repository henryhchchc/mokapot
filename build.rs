use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    compile_java_test_data();
}

fn compile_java_test_data() {
    if Command::new("javac").spawn().is_ok() {
        let build_path = env::var("OUT_DIR").unwrap();
        let test_data_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data"));
        let java_source_files: Vec<_> = fs::read_dir(test_data_path)
            .expect("Fail to open test data dir")
            .filter_map(|it| it.ok())
            .filter(|it| it.file_name().to_string_lossy().ends_with(".java"))
            .collect();
        java_source_files.into_iter().for_each(|java_file| {
            Command::new("javac")
                .arg("-g")
                .arg("-d")
                .arg(format!("{}/java_classes", build_path))
                .arg(java_file.path().to_string_lossy().to_string())
                .status()
                .unwrap();
        });
    }
}
