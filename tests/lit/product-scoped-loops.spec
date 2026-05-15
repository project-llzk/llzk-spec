// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/product-scoped-loops.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for tmpl::Prod {
  invariant for loop0(lb, i, ub, stride) {
    ensure i <= ub;
  }
}

// CHECK-DAG: "name": "tmpl::Prod"
// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "loop0"
