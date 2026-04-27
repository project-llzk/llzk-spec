use assert_cmd::Command;
use predicates::prelude::*;
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
fn succeeds_for_valid_spec() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("valid.spec"))
        .arg(fixture_path("valid.llzk"))
        .assert()
        .success();
}

#[test]
fn succeeds_for_valid_spec_with_scf_while() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("valid-while.spec"))
        .arg(fixture_path("valid-while.llzk"))
        .assert()
        .success();
}

#[test]
fn succeeds_when_spec_references_poly_param_symbol() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("poly-param.spec"))
        .arg(fixture_path("poly-param.llzk"))
        .assert()
        .success();
}

#[test]
fn succeeds_when_spec_references_poly_expr_symbol() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("poly-expr.spec"))
        .arg(fixture_path("poly-expr.llzk"))
        .assert()
        .success();
}

#[test]
fn fails_for_syntax_errors() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("broken.spec"))
        .arg(fixture_path("valid.llzk"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("syntax error"));
}

#[test]
fn fails_for_missing_contract_symbol() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("missing.spec"))
        .arg(fixture_path("valid.llzk"))
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unknown contract target `Missing`",
        ));
}

#[test]
fn fails_for_missing_loop_label() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("missing-loop.spec"))
        .arg(fixture_path("valid.llzk"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown loop label `loop2`"));
}

#[test]
fn emits_ast_to_stdout() {
    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(fixture_path("valid.spec"))
        .arg(fixture_path("valid.llzk"))
        .arg("--emit-ast")
        .arg("-")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"target\""));
}
