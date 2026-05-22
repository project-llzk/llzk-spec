// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate foo(x, y) {
  predicate bar(y) = y ** 2 < 2 ** 8

  return bar(x) || y;
}

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @foo(%[[VAL_0:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_1:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = function.call @bar(%[[VAL_0]]) : (!felt.type) -> i1
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = bool.or %[[VAL_2]], %[[VAL_1]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_3]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @bar(%[[VAL_4:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_4]], %[[VAL_5]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_7:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = felt.const  8
// CHECK-NEXT:      %[[VAL_9:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_7]], %[[VAL_8]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_10:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_6]], %[[VAL_9]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_10]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
