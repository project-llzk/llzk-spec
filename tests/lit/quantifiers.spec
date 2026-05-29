// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Foo {
  ensure forall i in 0..len(out), out[i] == out[i];
  ensure exists j in (0 + 1)..(len(out)), out[j] == out[j];
  ensure forall value in out, value == value;
}

// CHECK-DAG: "kind": "quantifier"
// CHECK-DAG: "quantifier_kind": "forall"
// CHECK-DAG: "quantifier_kind": "exists"
// CHECK-DAG: "kind": "range"
// CHECK-DAG: "kind": "expr"
// CHECK-DAG: "name": "i"
// CHECK-DAG: "name": "j"
// CHECK-DAG: "name": "value"
