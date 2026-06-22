// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/named-input-template-param.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Num2Bits::Num2Bits {
  require n == n;
  let copy = in;
  let same = copy;
  ensure true;
}

contract for LessThanPower::LessThanPower {
  require base == base;
  let res = in;
  let same = res;
  ensure true;
}

// CHECK-DAG: "name": "Num2Bits::Num2Bits"
// CHECK-DAG: "name": "LessThanPower::LessThanPower"
// CHECK-DAG: "kind": "require"
// CHECK-DAG: "kind": "let"
// CHECK-DAG: "kind": "boolean"
// CHECK-DAG: "name": "n"
// CHECK-DAG: "name": "base"
// CHECK-DAG: "name": "in"
// CHECK-DAG: "name": "res"
// CHECK-DAG: "name": "copy"
// CHECK-DAG: "name": "same"
