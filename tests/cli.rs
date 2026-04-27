use assert_cmd::Command;
use std::path::{Path, PathBuf};

/// Returns the absolute path to a checked-in CLI fixture.
fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cli")
        .join(name)
}

#[test]
fn succeeds_for_valid_spec_smoke() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("valid.spec"))
        .arg(fixture_path("valid.llzk"))
        .assert()
        .success();
}
