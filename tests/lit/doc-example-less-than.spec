// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/doc-example-less-than-num2bits.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for LessThan::LessThan {
  require n <= 252;
  ensure out == 1 ? arg[0][0] < arg[0][1] : arg[0][0] >= arg[0][1];
}

// CHECK-DAG: "name": "LessThan::LessThan"
// CHECK-DAG: "kind": "require"
// CHECK-DAG: "kind": "conditional"
// CHECK-DAG: "op": "le"
// CHECK-DAG: "op": "lt"
// CHECK-DAG: "op": "ge"
// CHECK-DAG: "kind": "arg"
// CHECK-DAG: "index": 0
// CHECK-DAG: "kind": "index"
