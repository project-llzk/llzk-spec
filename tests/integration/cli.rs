use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

const VALID_IR: &str = r#"
builtin.module {
  poly.template @Foo {
    struct.def @Foo {
      struct.member @out : !felt.type<\"bn128\"> {llzk.pub}
      function.def @compute(%arg0: !felt.type<\"bn128\">) -> !struct.type<@Foo::@Foo<[]>> attributes {function.allow_witness} {
        function.return
      }
      function.def @constrain(%arg0: !struct.type<@Foo::@Foo<[]>>, %arg1: !felt.type<\"bn128\">) attributes {function.allow_constraint} {
        %0 = scf.while (%arg2 = %arg1) : (!felt.type<\"bn128\">) -> (!felt.type<\"bn128\">) attributes {loop_label = \"loop1\", induction_arg = \"0\"} {
          scf.condition(%arg1) %arg2 : !felt.type<\"bn128\">
        } do {
        ^bb0(%arg3: !felt.type<\"bn128\">):
          scf.yield %arg3 : !felt.type<\"bn128\">
        }
        function.return
      }
    }
  }
}
"#;

#[test]
fn succeeds_for_valid_spec() {
    let dir = tempdir().expect("tempdir");
    let spec = dir.path().join("valid.spec");
    let ir = dir.path().join("valid.mlir");
    fs::write(&spec, "contract for Foo { ensure out == 0; }").expect("write spec");
    fs::write(&ir, VALID_IR).expect("write ir");

    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(&spec)
        .arg(&ir)
        .assert()
        .success();
}

#[test]
fn fails_for_syntax_errors() {
    let dir = tempdir().expect("tempdir");
    let spec = dir.path().join("broken.spec");
    let ir = dir.path().join("valid.mlir");
    fs::write(&spec, "contract for Foo { ensure ; }").expect("write spec");
    fs::write(&ir, VALID_IR).expect("write ir");

    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(&spec)
        .arg(&ir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("syntax error"));
}

#[test]
fn fails_for_missing_contract_symbol() {
    let dir = tempdir().expect("tempdir");
    let spec = dir.path().join("missing.spec");
    let ir = dir.path().join("valid.mlir");
    fs::write(&spec, "contract for Missing { ensure out == 0; }").expect("write spec");
    fs::write(&ir, VALID_IR).expect("write ir");

    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(&spec)
        .arg(&ir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown contract target `Missing`"));
}

#[test]
fn fails_for_missing_loop_label() {
    let dir = tempdir().expect("tempdir");
    let spec = dir.path().join("missing-loop.spec");
    let ir = dir.path().join("valid.mlir");
    fs::write(
        &spec,
        "contract for Foo { invariant for loop2(i) { ensure out == i; } }",
    )
    .expect("write spec");
    fs::write(&ir, VALID_IR).expect("write ir");

    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(&spec)
        .arg(&ir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown loop label `loop2`"));
}

#[test]
fn emits_ast_to_stdout() {
    let dir = tempdir().expect("tempdir");
    let spec = dir.path().join("ast.spec");
    let ir = dir.path().join("valid.mlir");
    fs::write(&spec, "contract for Foo { ensure out == 0; }").expect("write spec");
    fs::write(&ir, VALID_IR).expect("write ir");

    Command::cargo_bin("llzk-spec")
        .expect("binary")
        .arg(&spec)
        .arg(&ir)
        .arg("--emit-ast")
        .arg("-")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"target\""));
}
