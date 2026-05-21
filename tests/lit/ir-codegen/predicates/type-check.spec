// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate felts_and_bool(f1, f2, b1) = ((f1 + f2) == 0) && b1

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @felts_and_bool(%[[VAL_0:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_1:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_2:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = felt.add %[[VAL_0]], %[[VAL_1]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_4:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_3]], %[[VAL_4]]) : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = bool.and %[[VAL_5]], %[[VAL_2]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_6]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
