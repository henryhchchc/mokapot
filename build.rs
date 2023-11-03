use std::{env, process::Command};

fn main() {

    #[cfg(test)]
    compile_java_test_data();
}

#[cfg(test)]
fn compile_java_test_data() {
    let build_path = env::var("OUT_DIR").unwrap();
    Command::new("javac")
        .args(["--source", "20"])
        .arg("-d")
        .arg(format!("{}/java_classes", build_path))
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/MyClass.java"
        ))
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}
