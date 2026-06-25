// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/free-function-named-result.llzk --emit=ir | FileCheck %s
// END.

contract for foo {
  ensure out == $res[0];
}

// CHECK-LABEL: verif.contract @foo$contract$0 for @foo
// CHECK-SAME: (%[[ARG:[0-9a-zA-Z_\.]+]]: !felt.type, %[[RES:[0-9a-zA-Z_\.]+]]: !felt.type {function.res_name = "out"})
// CHECK: %[[COND:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[RES]], %[[RES]]) : !felt.type, !felt.type
// CHECK-NEXT: verif.ensure_compute %[[COND]]
// CHECK-NEXT: verif.ensure_constrain %[[COND]]
