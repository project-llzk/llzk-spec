// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Foo {
  let a = nondet;
  let b = nondet;
  let c = nondet;

  ensure a + b << c;
  ensure a << b < c;
  ensure a << b >> c;
}

// CHECK: "op": "shl"
// CHECK: "left": {
// CHECK: "op": "add"
// CHECK: "right": {
// CHECK: "name": "c"

// CHECK: "op": "lt"
// CHECK: "left": {
// CHECK: "op": "shl"
// CHECK: "right": {
// CHECK: "name": "c"

// CHECK: "op": "shr"
// CHECK: "left": {
// CHECK: "op": "shl"
