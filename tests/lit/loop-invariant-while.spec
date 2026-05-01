// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-while.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  invariant for loop0(i) {
    step i == old(i) + 1;
    ensure out == out;
  }
}

// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "loop0"
// CHECK-DAG: "name": "i"
// CHECK-DAG: "kind": "step"
// CHECK-DAG: "kind": "old"
