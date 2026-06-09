// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate identity(x) = x

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
// CHECK-NEXT:    function.def @identity(%[[VAL_6:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      function.return %[[VAL_6]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
