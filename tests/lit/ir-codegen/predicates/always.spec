// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate always() = true

// CHECK-DAG: "kind": "predicate"
// CHECK-DAG: "name": "always"
// CHECK-DAG: "name": "equals"
// CHECK-DAG: "name": "local"
// CHECK-DAG: "kind": "return"
// CHECK-DAG: "kind": "call"
// CHECK-DAG: "callee"
