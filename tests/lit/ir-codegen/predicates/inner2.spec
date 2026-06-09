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
// CHECK-NEXT:    struct.def @Foo {
// CHECK-NEXT:      struct.member @out : index
// CHECK-NEXT:      function.def @compute() -> !struct.type<@Foo<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
// CHECK-NEXT:        %[[VAL_0:[0-9a-zA-Z_\.]+]] = struct.new : <@Foo<[]>>
// CHECK-NEXT:        function.return %[[VAL_0]] : !struct.type<@Foo<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_1:[0-9a-zA-Z_\.]+]]: !struct.type<@Foo<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
// CHECK-NEXT:        %[[VAL_2:[0-9a-zA-Z_\.]+]] = arith.constant 0 : index
// CHECK-NEXT:        %[[VAL_3:[0-9a-zA-Z_\.]+]] = arith.constant 1 : index
// CHECK-NEXT:        %[[VAL_4:[0-9a-zA-Z_\.]+]] = arith.constant 2 : index
// CHECK-NEXT:        scf.for %[[VAL_5:[0-9a-zA-Z_\.]+]] = %[[VAL_2]] to %[[VAL_4]] step %[[VAL_3]] {
// CHECK-NEXT:        }
// CHECK-NEXT:        function.return
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @bar(%[[VAL_6:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_7:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = bool.and %[[VAL_6]], %[[VAL_7]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_8]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @foo(%[[VAL_9:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_10:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = function.call @bar_0(%[[VAL_9]]) : (!felt.type) -> i1
// CHECK-NEXT:      %[[VAL_12:[0-9a-zA-Z_\.]+]] = bool.or %[[VAL_11]], %[[VAL_10]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_12]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @bar_0(%[[VAL_13:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_14:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:      %[[VAL_15:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_13]], %[[VAL_14]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_16:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:      %[[VAL_17:[0-9a-zA-Z_\.]+]] = felt.const  8
// CHECK-NEXT:      %[[VAL_18:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_16]], %[[VAL_17]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_19:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_15]], %[[VAL_18]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_19]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
