// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/free-function.llzk --emit=ir | FileCheck %s
// END.

contract for foo {
  ensure $arg[0] == $res[0];
  ensure $res[0] == $arg[0];
}

// CHECK-LABEL: verif.contract @foo$contract$0 for @foo
// CHECK-SAME: (%[[ARG:[0-9a-zA-Z_\.]+]]: !felt.type, %[[RES:[0-9a-zA-Z_\.]+]]: !felt.type)
// CHECK: %[[COND0:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[ARG]], %[[RES]]) : !felt.type, !felt.type
// CHECK-NEXT: verif.ensure_compute %[[COND0]]
// CHECK-NEXT: verif.ensure_constrain %[[COND0]]
// CHECK: %[[COND1:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[RES]], %[[ARG]]) : !felt.type, !felt.type
// CHECK-NEXT: verif.ensure_compute %[[COND1]]
// CHECK-NEXT: verif.ensure_constrain %[[COND1]]
