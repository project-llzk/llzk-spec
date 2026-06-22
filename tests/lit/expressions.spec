// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Foo {
  let a = nondet;
  let b = nondet;
  let c = nondet;
  let d = nondet;

  require true;
  ensure !false || a != b && c <= a;
  ensure a >= b ? -a == b : out[0] == len(out);
  ensure (a / 1) % 2 == (b & c) ** 3;
  ensure (a << b) == (c >> d);
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
// CHECK-DAG: "op": "shl"
// CHECK-DAG: "op": "shr"
