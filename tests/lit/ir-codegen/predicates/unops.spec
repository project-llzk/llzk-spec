// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

// Extra comparison since predicates must return a boolean.
predicate neg(x) = (-x) != 0

predicate not(x) = !x

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @neg(%[[VAL_0:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_1:[0-9a-zA-Z_\.]+]] = felt.neg %[[VAL_0]] : !felt.type
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = bool.cmp ne(%[[VAL_1]], %[[VAL_2]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_3]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @not(%[[VAL_4:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = bool.not %[[VAL_4]] : i1
// CHECK-NEXT:      function.return %[[VAL_5]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
