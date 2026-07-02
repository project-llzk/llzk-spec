// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/product-named-input.llzk --emit=ir | FileCheck %s
// END.

contract for tmpl::Prod {
  let copy = input;
  ensure copy == $arg[0];
}

// CHECK-LABEL: verif.contract @tmpl$Prod$contract$0 for @tmpl::@Prod
// CHECK: %[[INPUT:[0-9a-zA-Z_\.]+]]: index {function.arg_name = "input"}
// CHECK: %[[LHS:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[INPUT]] : index, !felt.type
// CHECK-NEXT: %[[RHS:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[INPUT]] : index, !felt.type
// CHECK-NEXT: %[[COND:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[LHS]], %[[RHS]]) : !felt.type, !felt.type
// CHECK-NEXT: verif.ensure_compute %[[COND]]
// CHECK-NEXT: verif.ensure_constrain %[[COND]]
