// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/product-scoped-loops.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/product-scoped-loops.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Prod {
  invariant for loop0(lb, i, ub, stride) {
    ensure i <= ub;
  }
}

// CHECK-DAG: "name": "Prod"
// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "loop0"
