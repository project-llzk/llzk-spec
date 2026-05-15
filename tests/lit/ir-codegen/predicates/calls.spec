// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate not(x) = !x

predicate identity(x) = not(not(x))

predicate recursive(x) = x ? true : recursive(true)


// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @not(%[[VAL_0:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_1:[0-9a-zA-Z_\.]+]] = bool.not %[[VAL_0]] : i1
// CHECK-NEXT:      function.return %[[VAL_1]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @identity(%[[VAL_2:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = function.call @not(%[[VAL_2]]) : (i1) -> i1
// CHECK-NEXT:      %[[VAL_4:[0-9a-zA-Z_\.]+]] = function.call @not(%[[VAL_3]]) : (i1) -> i1
// CHECK-NEXT:      function.return %[[VAL_4]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @recursive(%[[VAL_5:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = scf.if %[[VAL_5]] -> (i1) {
// CHECK-NEXT:        %[[VAL_7:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:        scf.yield %[[VAL_7]] : i1
// CHECK-NEXT:      } else {
// CHECK-NEXT:        %[[VAL_8:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:        %[[VAL_9:[0-9a-zA-Z_\.]+]] = function.call @recursive(%[[VAL_8]]) : (i1) -> i1
// CHECK-NEXT:        scf.yield %[[VAL_9]] : i1
// CHECK-NEXT:      }
// CHECK-NEXT:      function.return %[[VAL_6]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
