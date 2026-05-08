// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-labeled-for.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  invariant for for_label(lb, i, ub, stride) {
    ensure out == out;
  }
}

// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "for_label"
