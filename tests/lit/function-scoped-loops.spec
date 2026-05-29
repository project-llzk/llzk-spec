// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/function-scoped-loops.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Foo {
  invariant for loop0(lb, i, ub, stride) {
    ensure i >= lb;
  }

  invariant for loop1(lb, i, ub, stride) {
    ensure i <= ub;
  }
}

// CHECK-DAG: "name": "loop0"
// CHECK-DAG: "name": "loop1"
