// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  let a = nondet;
  let b = nondet;
  let c = nondet;
  let d = nondet;

  ensure d - ((a + b) * c) == 0;
}

// CHECK: "op": "eq"
// CHECK: "op": "sub"
// CHECK: "name": "d"
// CHECK: "op": "mul"
// CHECK: "op": "add"
// CHECK: "name": "a"
// CHECK: "name": "b"
// CHECK: "name": "c"
