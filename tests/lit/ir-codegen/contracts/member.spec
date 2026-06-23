// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/member-access.llzk --emit=ir  | FileCheck %s
// END.

contract for Parent {
  ensure child.pub_out == out;
  ensure children[0].pub_out == out;
  ensure arr[0] == out;
  ensure pod.flag == out;
}

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    verif.contract @Parent$contract$0 for @Parent (%[[VAL_0:[0-9a-zA-Z_\.]+]]: !struct.type<@Parent<[]>>) {
// CHECK-NEXT:      %[[VAL_1:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@out] : <@Parent<[]>>, index
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@child] : <@Parent<[]>>, !struct.type<@Inner<[]>>
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@children] : <@Parent<[]>>, !array.type<2 x !struct.type<@Inner<[]>>>
// CHECK-NEXT:      %[[VAL_4:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@arr] : <@Parent<[]>>, !array.type<2 x index>
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@pod] : <@Parent<[]>>, !pod.type<[@count: index, @flag: index]>
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_2]][@pub_out] : <@Inner<[]>>, index
// CHECK-NEXT:      %[[VAL_7:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_6]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_1]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_9:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_7]], %[[VAL_8]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_9]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_9]]
// CHECK-NEXT:      %[[VAL_10:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = cast.toindex %[[VAL_10]] : !felt.type
// CHECK-NEXT:      %[[VAL_12:[0-9a-zA-Z_\.]+]] = array.read %[[VAL_3]]{{\[}}%[[VAL_11]]] : <2 x !struct.type<@Inner<[]>>>, !struct.type<@Inner<[]>>
// CHECK-NEXT:      %[[VAL_13:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_12]][@pub_out] : <@Inner<[]>>, index
// CHECK-NEXT:      %[[VAL_14:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_13]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_15:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_1]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_16:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_14]], %[[VAL_15]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_16]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_16]]
// CHECK-NEXT:      %[[VAL_17:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:      %[[VAL_18:[0-9a-zA-Z_\.]+]] = cast.toindex %[[VAL_17]] : !felt.type
// CHECK-NEXT:      %[[VAL_19:[0-9a-zA-Z_\.]+]] = array.read %[[VAL_4]]{{\[}}%[[VAL_18]]] : <2 x index>, index
// CHECK-NEXT:      %[[VAL_20:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_19]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_21:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_1]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_22:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_20]], %[[VAL_21]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_22]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_22]]
// CHECK-NEXT:      %[[VAL_23:[0-9a-zA-Z_\.]+]] = pod.read %[[VAL_5]][@flag] : <[@count: index, @flag: index]>, index
// CHECK-NEXT:      %[[VAL_24:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_23]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_25:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_1]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_26:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_24]], %[[VAL_25]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_26]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_26]]
// CHECK-NEXT:    }
// CHECK-NEXT:    struct.def @Inner {
// CHECK-NEXT:      struct.member @pub_out : index {llzk.pub}
// CHECK-NEXT:      struct.member @secret : index
// CHECK-NEXT:      struct.member @arr_pub : !array.type<2 x index> {llzk.pub}
// CHECK-NEXT:      function.def @compute() -> !struct.type<@Inner<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
// CHECK-NEXT:        %[[VAL_27:[0-9a-zA-Z_\.]+]] = struct.new : <@Inner<[]>>
// CHECK-NEXT:        function.return %[[VAL_27]] : !struct.type<@Inner<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_28:[0-9a-zA-Z_\.]+]]: !struct.type<@Inner<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
// CHECK-NEXT:        function.return
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:    struct.def @Parent {
// CHECK-NEXT:      struct.member @out : index
// CHECK-NEXT:      struct.member @child : !struct.type<@Inner<[]>>
// CHECK-NEXT:      struct.member @children : !array.type<2 x !struct.type<@Inner<[]>>>
// CHECK-NEXT:      struct.member @arr : !array.type<2 x index>
// CHECK-NEXT:      struct.member @pod : !pod.type<[@count: index, @flag: index]>
// CHECK-NEXT:      function.def @compute() -> !struct.type<@Parent<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
// CHECK-NEXT:        %[[VAL_29:[0-9a-zA-Z_\.]+]] = struct.new : <@Parent<[]>>
// CHECK-NEXT:        function.return %[[VAL_29]] : !struct.type<@Parent<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_30:[0-9a-zA-Z_\.]+]]: !struct.type<@Parent<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
// CHECK-NEXT:        function.return
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:  }
