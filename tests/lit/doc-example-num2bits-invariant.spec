// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/doc-example-less-than-num2bits.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Num2Bits::Num2Bits {
  invariant for loop1(e2, i, lc1) {
    decreases n - i;
    step lc1 == old(lc1) + out[i] * e2;
    ensure out[i] == 0 || out[i] == 1;
    ensure $arg[0] & (2 ** i) == out[i] * (2 ** i);
  }
}

// CHECK-DAG: "name": "Num2Bits::Num2Bits"
// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "loop1"
// CHECK-DAG: "name": "e2"
// CHECK-DAG: "name": "lc1"
// CHECK-DAG: "kind": "decreases"
// CHECK-DAG: "kind": "step"
// CHECK-DAG: "kind": "old"
// CHECK-DAG: "op": "or"
// CHECK-DAG: "op": "bit_and"
// CHECK-DAG: "op": "pow"
// CHECK-DAG: "kind": "arg"
// CHECK-DAG: "index": 0
// CHECK-DAG: "kind": "index"
