// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit-ast - --format json | FileCheck %s
// END.

predicate always() = true

predicate equals(a, b) {
  return a == b;
}

contract for Foo {
  predicate local(x) = equals(x, out)

  ensure always();
  ensure equals(out, out);
  ensure local(out);
}

// CHECK-DAG: "kind": "predicate"
// CHECK-DAG: "name": "always"
// CHECK-DAG: "name": "equals"
// CHECK-DAG: "name": "local"
// CHECK-DAG: "kind": "return"
// CHECK-DAG: "kind": "call"
// CHECK-DAG: "callee"
