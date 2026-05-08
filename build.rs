use glob::glob;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Generates the file `discovered_tests.in` in the output directory, containing
/// test functions for each `.spec` file found in the `tests/lit` directory.
/// Each test function is named based on the file path, with slashes replaced
/// by underscores, and is set up to call `lit_test` with the file's contents.
fn main() {
    println!("cargo:rerun-if-changed=tests/lit");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    let dest_path = Path::new(&out_dir).join("discovered_lit_tests.rs");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let pattern = manifest_dir.join("tests/lit/**/*.spec");
    let pattern = pattern.to_string_lossy();

    let mut paths = glob(&pattern)
        .expect("valid lit test glob")
        .map(|entry| entry.expect("lit test path"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut generated = String::new();
    for path in paths {
        let rel_path = path
            .strip_prefix(&manifest_dir)
            .expect("lit test under manifest dir")
            .to_string_lossy()
            .replace('\\', "/");
        println!("cargo:rerun-if-changed={rel_path}");

        let test_name = test_name(&rel_path);
        generated.push_str(&format!(
            r#"
#[test]
fn {test_name}() -> LitResult<()> {{
    lit_test(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "{rel_path}")),
        "{rel_path}",
    )
}}
"#
        ));
    }

    write_file(dest_path, generated.as_bytes());
}

fn test_name(path: &str) -> String {
    let base = path.strip_prefix("tests/lit/").unwrap_or(path);
    let base = base.strip_suffix(".spec").unwrap_or(base);
    let mut name = String::from("lit_");
    for ch in base.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
        } else {
            name.push('_');
        }
    }
    name
}

fn write_file<P: AsRef<Path>>(path: P, text: &[u8]) {
    let mut file = File::create(path).expect("create generated lit tests file");
    file.write_all(text)
        .expect("write generated lit tests file");
}
