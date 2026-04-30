// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/nested-modules.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/nested-modules.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Bar::Foo {
  invariant for same(lb, i, ub, stride) {
    ensure out == out;
  }
}

// CHECK-DAG: "target"
// CHECK-DAG: "Bar::Foo"
// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "same"
