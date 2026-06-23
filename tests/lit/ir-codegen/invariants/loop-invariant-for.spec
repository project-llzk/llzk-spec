// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir | FileCheck %s
// END.

contract for Foo {
  invariant for loop0(lb, i, ub, stride) {
    increases i;
    decreases ub - i;
    step i == old(i) + stride;
    ensure out == out;
  }
}

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    verif.contract @Foo$contract$0 for @Foo (%[[VAL_0:[0-9a-zA-Z_\.]+]]: !struct.type<@Foo<[]>>) {
// CHECK-NEXT:      %[[VAL_1:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@out] : <@Foo<[]>>, index
// CHECK-NEXT:      verif.invariant for @loop0(%[[VAL_2:[0-9a-zA-Z_\.]+]]: index, %[[VAL_3:[0-9a-zA-Z_\.]+]]: index, %[[VAL_4:[0-9a-zA-Z_\.]+]]: index, %[[VAL_5:[0-9a-zA-Z_\.]+]]: index) {
// CHECK-NEXT:        %[[VAL_6:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_3]] : index, !felt.type
// CHECK-NEXT:        verif.increases %[[VAL_6]]
// CHECK-NEXT:        %[[VAL_7:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_4]] : index, !felt.type
// CHECK-NEXT:        %[[VAL_8:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_3]] : index, !felt.type
// CHECK-NEXT:        %[[VAL_9:[0-9a-zA-Z_\.]+]] = felt.sub %[[VAL_7]], %[[VAL_8]] : !felt.type, !felt.type
// CHECK-NEXT:        verif.decreases %[[VAL_9]]
// CHECK-NEXT:        verif.step {
// CHECK-NEXT:          %[[VAL_10:[0-9a-zA-Z_\.]+]] = verif.old %[[VAL_3]] : index
// CHECK-NEXT:          %[[VAL_11:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_10]] : index, !felt.type
// CHECK-NEXT:          %[[VAL_12:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_5]] : index, !felt.type
// CHECK-NEXT:          %[[VAL_13:[0-9a-zA-Z_\.]+]] = felt.add %[[VAL_11]], %[[VAL_12]] : !felt.type, !felt.type
// CHECK-NEXT:          %[[VAL_14:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_3]] : index, !felt.type
// CHECK-NEXT:          %[[VAL_15:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_14]], %[[VAL_13]]) : !felt.type, !felt.type
// CHECK-NEXT:          verif.step.yield %[[VAL_15]]
// CHECK-NEXT:        }
// CHECK-NEXT:        %[[VAL_16:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_1]] : index, !felt.type
// CHECK-NEXT:        %[[VAL_17:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_1]] : index, !felt.type
// CHECK-NEXT:        %[[VAL_18:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_16]], %[[VAL_17]]) : !felt.type, !felt.type
// CHECK-NEXT:        verif.ensure_compute %[[VAL_18]]
// CHECK-NEXT:        verif.ensure_constrain %[[VAL_18]]
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:    struct.def @Foo {
// CHECK-NEXT:      struct.member @out : index
// CHECK-NEXT:      function.def @compute() -> !struct.type<@Foo<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
// CHECK-NEXT:        %[[VAL_19:[0-9a-zA-Z_\.]+]] = struct.new : <@Foo<[]>>
// CHECK-NEXT:        function.return %[[VAL_19]] : !struct.type<@Foo<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_20:[0-9a-zA-Z_\.]+]]: !struct.type<@Foo<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
// CHECK-NEXT:        %[[VAL_21:[0-9a-zA-Z_\.]+]] = arith.constant 0 : index
// CHECK-NEXT:        %[[VAL_22:[0-9a-zA-Z_\.]+]] = arith.constant 1 : index
// CHECK-NEXT:        %[[VAL_23:[0-9a-zA-Z_\.]+]] = arith.constant 2 : index
// CHECK-NEXT:        scf.for %[[VAL_24:[0-9a-zA-Z_\.]+]] = %[[VAL_21]] to %[[VAL_23]] step %[[VAL_22]] {
// CHECK-NEXT:        } {loop_label = "loop0"}
// CHECK-NEXT:        function.return
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:  }
