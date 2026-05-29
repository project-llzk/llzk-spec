// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

// Outer predicate 'bar' has type 'Bool -> Bool'
predicate bar(w) = w && true

predicate foo(x, y) {
  // Inner predicate 'bar' has type 'Felt -> Bool'
  predicate bar(y) = y ** 2 < 2 ** 8

  return bar(x) || y;
}

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @bar(%[[VAL_0:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_1:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = bool.and %[[VAL_0]], %[[VAL_1]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_2]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @foo(%[[VAL_3:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_4:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = function.call @bar_0(%[[VAL_3]]) : (!felt.type) -> i1
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = bool.or %[[VAL_5]], %[[VAL_4]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_6]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @bar_0(%[[VAL_7:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:      %[[VAL_9:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_7]], %[[VAL_8]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_10:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = felt.const  8
// CHECK-NEXT:      %[[VAL_12:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_10]], %[[VAL_11]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_13:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_9]], %[[VAL_12]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_13]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
