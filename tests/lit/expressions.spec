// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  let a = nondet;
  let b = nondet;
  let c = nondet;

  require true;
  ensure !false || a != b && c <= a;
  ensure a >= b ? -a == b : out[0] == len(out);
  ensure (a / 1) % 2 == (b & c) ** 3;
}

// CHECK-DAG: "kind": "require"
// CHECK-DAG: "kind": "conditional"
// CHECK-DAG: "kind": "unary"
// CHECK-DAG: "kind": "index"
// CHECK-DAG: "kind": "len"
// CHECK-DAG: "op": "or"
// CHECK-DAG: "op": "and"
// CHECK-DAG: "op": "ne"
// CHECK-DAG: "op": "le"
// CHECK-DAG: "op": "ge"
// CHECK-DAG: "op": "div"
// CHECK-DAG: "op": "mod"
// CHECK-DAG: "op": "bit_and"
// CHECK-DAG: "op": "pow"
