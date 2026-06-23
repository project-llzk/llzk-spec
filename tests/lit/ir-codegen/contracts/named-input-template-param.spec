// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/named-input-template-param.llzk --emit=ir | FileCheck %s
// END.

contract for Num2Bits::Num2Bits {
  require n == n;
  let copy = in;
  let same = arg[0];
  ensure true;
}

contract for LessThanPower::LessThanPower {
  require base == base;
  let res = in;
  let same = arg[0];
  ensure true;
}

// CHECK-LABEL: verif.contract @Num2Bits$Num2Bits$contract$0 for @Num2Bits::@Num2Bits
// CHECK: %arg1: !felt.type<"bn128"> {function.arg_name = "in"}
// CHECK: %[[N:[0-9a-zA-Z_\.]+]] = poly.read_const @n
// CHECK: verif.require_compute
// CHECK-LABEL: verif.contract @LessThanPower$LessThanPower$contract$1 for @LessThanPower::@LessThanPower
// CHECK: %arg1: !felt.type<"bn128"> {function.arg_name = "in"}
// CHECK: %[[BASE:[0-9a-zA-Z_\.]+]] = poly.read_const @base
// CHECK: verif.ensure_compute
