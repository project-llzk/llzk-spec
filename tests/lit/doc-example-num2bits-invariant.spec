// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/doc-example-less-than-num2bits.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Num2Bits {
  invariant for loop1(i) {
    ensure out[i] == 0 || out[i] == 1;
    ensure arg[0] & (2 ** i) == out[i] * (2 ** i);
  }
}

// CHECK-DAG: "name": "Num2Bits"
// CHECK-DAG: "kind": "invariant"
// CHECK-DAG: "name": "loop1"
// CHECK-DAG: "op": "or"
// CHECK-DAG: "op": "bit_and"
// CHECK-DAG: "op": "pow"
// CHECK-DAG: "kind": "arg"
// CHECK-DAG: "index": 0
// CHECK-DAG: "kind": "index"
