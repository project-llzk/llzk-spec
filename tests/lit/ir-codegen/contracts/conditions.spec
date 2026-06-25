// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/unnested.llzk --emit=ir  | FileCheck %s
// END.

predicate orp(x, y) = x || y

contract for Foo {
  require arg[0] == 3;
  ensure out < 4;

  compute {
    require arg[0] != 8;
    ensure orp(true, false);
  }

  constrain {
    require arg[0] > 2;
    ensure out < 10;
  }
}

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    verif.contract @Foo$contract$0 for @Foo (%[[VAL_0:[0-9a-zA-Z_\.]+]]: !struct.type<@Foo<[]>>, %[[VAL_1:[0-9a-zA-Z_\.]+]]: !felt.type) {
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@out] : <@Foo<[]>>, index
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = felt.const  3
// CHECK-NEXT:      %[[VAL_4:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_1]], %[[VAL_3]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.require_compute %[[VAL_4]]
// CHECK-NEXT:      verif.require_constrain %[[VAL_4]]
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = felt.const  4
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_2]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_7:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_6]], %[[VAL_5]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_7]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_7]]
// CHECK-NEXT:      scf.execute_region {
// CHECK-NEXT:        scf.execute_region {
// CHECK-NEXT:          %[[VAL_8:[0-9a-zA-Z_\.]+]] = felt.const  8
// CHECK-NEXT:          %[[VAL_9:[0-9a-zA-Z_\.]+]] = bool.cmp ne(%[[VAL_1]], %[[VAL_8]]) : !felt.type, !felt.type
// CHECK-NEXT:          verif.require_compute %[[VAL_9]]
// CHECK-NEXT:          %[[VAL_10:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:          %[[VAL_11:[0-9a-zA-Z_\.]+]] = arith.constant false
// CHECK-NEXT:          %[[VAL_12:[0-9a-zA-Z_\.]+]] = function.call @orp(%[[VAL_10]], %[[VAL_11]]) : (i1, i1) -> i1
// CHECK-NEXT:          verif.ensure_compute %[[VAL_12]]
// CHECK-NEXT:          scf.yield
// CHECK-NEXT:        }
// CHECK-NEXT:        scf.yield
// CHECK-NEXT:      }
// CHECK-NEXT:      scf.execute_region {
// CHECK-NEXT:        scf.execute_region {
// CHECK-NEXT:          %[[VAL_13:[0-9a-zA-Z_\.]+]] = felt.const  2
// CHECK-NEXT:          %[[VAL_14:[0-9a-zA-Z_\.]+]] = bool.cmp gt(%[[VAL_1]], %[[VAL_13]]) : !felt.type, !felt.type
// CHECK-NEXT:          verif.require_constrain %[[VAL_14]]
// CHECK-NEXT:          %[[VAL_15:[0-9a-zA-Z_\.]+]] = felt.const  10
// CHECK-NEXT:          %[[VAL_16:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_2]] : index, !felt.type
// CHECK-NEXT:          %[[VAL_17:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_16]], %[[VAL_15]]) : !felt.type, !felt.type
// CHECK-NEXT:          verif.ensure_constrain %[[VAL_17]]
// CHECK-NEXT:          scf.yield
// CHECK-NEXT:        }
// CHECK-NEXT:        scf.yield
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:    struct.def @Foo {
// CHECK-NEXT:      struct.member @out : index
// CHECK-NEXT:      function.def @compute(%[[VAL_18:[0-9a-zA-Z_\.]+]]: !felt.type) -> !struct.type<@Foo<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
// CHECK-NEXT:        %[[VAL_19:[0-9a-zA-Z_\.]+]] = struct.new : <@Foo<[]>>
// CHECK-NEXT:        function.return %[[VAL_19]] : !struct.type<@Foo<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_20:[0-9a-zA-Z_\.]+]]: !struct.type<@Foo<[]>>, %[[VAL_21:[0-9a-zA-Z_\.]+]]: !felt.type) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
// CHECK-NEXT:        %[[VAL_22:[0-9a-zA-Z_\.]+]] = arith.constant 0 : index
// CHECK-NEXT:        %[[VAL_23:[0-9a-zA-Z_\.]+]] = arith.constant 1 : index
// CHECK-NEXT:        %[[VAL_24:[0-9a-zA-Z_\.]+]] = arith.constant 2 : index
// CHECK-NEXT:        scf.for %[[VAL_25:[0-9a-zA-Z_\.]+]] = %[[VAL_22]] to %[[VAL_24]] step %[[VAL_23]] {
// CHECK-NEXT:        } {loop_label = "same"}
// CHECK-NEXT:        function.return
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @orp(%[[VAL_26:[0-9a-zA-Z_\.]+]]: i1, %[[VAL_27:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_28:[0-9a-zA-Z_\.]+]] = bool.or %[[VAL_26]], %[[VAL_27]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_28]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
