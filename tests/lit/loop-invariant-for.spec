// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  invariant for loop0(lb, i, ub, stride) {
    increases i;
    decreases ub - i;
    step i == old(i) + stride;
    ensure out == out;
  }
}

// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "loop0"
// CHECK-DAG: "name": "lb"
// CHECK-DAG: "name": "stride"
// CHECK-DAG: "kind": "increases"
// CHECK-DAG: "kind": "decreases"
// CHECK-DAG: "kind": "step"
// CHECK-DAG: "kind": "old"
