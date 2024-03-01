use mokapot::jvm::class;
use rayon::prelude::*;
use std::{fs, path::PathBuf, process::Command};
use tempdir::TempDir;

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("JAVA_HOME is not set")]
    JavaHomeNotSet,
    #[error("Failed to extract jdk module image")]
    ExtractionFail,
}

fn extract_jdk_module_image() -> Result<TempDir, TestError> {
    let java_home = std::env::var("JAVA_HOME").map_err(|_| TestError::JavaHomeNotSet)?;
    let jdk_modules = PathBuf::from(java_home).join("lib").join("modules");
    let temp = TempDir::new("mokapot_test").unwrap();
    Command::new("jimage")
        .arg("extract")
        .arg(format!("--dir={}", temp.path().to_string_lossy()))
        .arg(jdk_modules)
        .status()
        .map_err(|_| TestError::ExtractionFail)?;
    Ok(temp)
}

#[test]
#[ignore = "Takes long"]
fn parse_jdk_classes() {
    let extracted_modules_images = extract_jdk_module_image().unwrap();
    let class_files: Vec<_> = walkdir::WalkDir::new(extracted_modules_images.path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|it| it.path().extension().is_some_and(|it| it == "class"))
        .map(|it| it.into_path())
        .collect();

    class_files.into_par_iter().for_each(|class_file| {
        eprintln!("Parsing {:?}", class_file);
        let reader = fs::File::open(&class_file).unwrap();
        let buf_reader = std::io::BufReader::new(reader);
        let class = class::Class::from_reader(buf_reader);
        if let Err(e) = class {
            panic!("Failed to parse {:?}: {}", class_file, e);
        }
    });
}
