use mokapot::jvm::class;
use rayon::prelude::*;
use std::{env, fs, path::PathBuf};

#[test]
#[ignore = "Takes long"]
fn parse_jdk_classes() {
    let extracted_modules_images = env::var("JDK_CLASSES").unwrap();
    let extracted_modules_images = PathBuf::from(extracted_modules_images);
    let class_files: Vec<_> = walkdir::WalkDir::new(extracted_modules_images)
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
