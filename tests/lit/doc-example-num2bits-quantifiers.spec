// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/doc-example-less-than-num2bits.llzk --emit=ast --emit-format json | FileCheck %s
// END.

predicate bit_i_equals_out_i(in, out, i) {
  let bit_i = (in & 2**i) != 0 ? 1 : 0;
  return bit_i == out[i];
}

contract for Num2Bits::Num2Bits {
  ensure forall o in out, o == 0 || o == 1;
  ensure forall i in 0..n, bit_i_equals_out_i($arg[0], out, i);
}

// CHECK-DAG: "name": "bit_i_equals_out_i"
// CHECK-DAG: "kind": "return"
// CHECK-DAG: "kind": "conditional"
// CHECK-DAG: "op": "bit_and"
// CHECK-DAG: "op": "pow"
// CHECK-DAG: "quantifier_kind": "forall"
// CHECK-DAG: "kind": "range"
// CHECK-DAG: "kind": "call"
// CHECK-DAG: "kind": "arg"
// CHECK-DAG: "index": 0
