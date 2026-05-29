// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate nondet_pred() = nondet == 0

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @nondet_pred() -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_0:[0-9a-zA-Z_\.]+]] = llzk.nondet : !felt.type
// CHECK-NEXT:      %[[VAL_1:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_0]], %[[VAL_1]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_2]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
