// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/member-access.llzk --emit=ir  | FileCheck %s
// END.

contract for Parent {
  ensure child.pub_out == out;
  // Casting from a felt to an index is not allowed inside a `verif.contract` op.
  // Thus, expressions like `arr[0]` wont work because `0 : !felt.type` but the 
  // `array.read` operation expects indices of `index` type. And the `cast.toindex` op 
  // is not allowed (yet)
  ensure children[pod.count].pub_out == out;
  ensure arr[pod.count] == out;
  ensure pod.flag == out;
}

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    struct.def @Inner {
// CHECK-NEXT:      struct.member @pub_out : index {llzk.pub}
// CHECK-NEXT:      struct.member @secret : index
// CHECK-NEXT:      struct.member @arr_pub : !array.type<2 x index> {llzk.pub}
// CHECK-NEXT:      function.def @compute() -> !struct.type<@Inner<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
// CHECK-NEXT:        %[[VAL_0:[0-9a-zA-Z_\.]+]] = struct.new : <@Inner<[]>>
// CHECK-NEXT:        function.return %[[VAL_0]] : !struct.type<@Inner<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_1:[0-9a-zA-Z_\.]+]]: !struct.type<@Inner<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
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
// CHECK-NEXT:        %[[VAL_2:[0-9a-zA-Z_\.]+]] = struct.new : <@Parent<[]>>
// CHECK-NEXT:        function.return %[[VAL_2]] : !struct.type<@Parent<[]>>
// CHECK-NEXT:      }
// CHECK-NEXT:      function.def @constrain(%[[VAL_3:[0-9a-zA-Z_\.]+]]: !struct.type<@Parent<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
// CHECK-NEXT:        function.return
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:    verif.contract @Parent$contract$0 for @Parent (%[[VAL_4:[0-9a-zA-Z_\.]+]]: !struct.type<@Parent<[]>>) {
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_4]][@out] : <@Parent<[]>>, index
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_4]][@child] : <@Parent<[]>>, !struct.type<@Inner<[]>>
// CHECK-NEXT:      %[[VAL_7:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_4]][@children] : <@Parent<[]>>, !array.type<2 x !struct.type<@Inner<[]>>>
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_4]][@arr] : <@Parent<[]>>, !array.type<2 x index>
// CHECK-NEXT:      %[[VAL_9:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_4]][@pod] : <@Parent<[]>>, !pod.type<[@count: index, @flag: index]>
// CHECK-NEXT:      %[[VAL_10:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_6]][@pub_out] : <@Inner<[]>>, index
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_10]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_12:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_5]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_13:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_11]], %[[VAL_12]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_13]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_13]]
// CHECK-NEXT:      %[[VAL_14:[0-9a-zA-Z_\.]+]] = pod.read %[[VAL_9]][@count] : <[@count: index, @flag: index]>, index
// CHECK-NEXT:      %[[VAL_15:[0-9a-zA-Z_\.]+]] = array.read %[[VAL_7]]{{\[}}%[[VAL_14]]] : <2 x !struct.type<@Inner<[]>>>, !struct.type<@Inner<[]>>
// CHECK-NEXT:      %[[VAL_16:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_15]][@pub_out] : <@Inner<[]>>, index
// CHECK-NEXT:      %[[VAL_17:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_16]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_18:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_5]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_19:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_17]], %[[VAL_18]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_19]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_19]]
// CHECK-NEXT:      %[[VAL_20:[0-9a-zA-Z_\.]+]] = pod.read %[[VAL_9]][@count] : <[@count: index, @flag: index]>, index
// CHECK-NEXT:      %[[VAL_21:[0-9a-zA-Z_\.]+]] = array.read %[[VAL_8]]{{\[}}%[[VAL_20]]] : <2 x index>, index
// CHECK-NEXT:      %[[VAL_22:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_21]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_23:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_5]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_24:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_22]], %[[VAL_23]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_24]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_24]]
// CHECK-NEXT:      %[[VAL_25:[0-9a-zA-Z_\.]+]] = pod.read %[[VAL_9]][@flag] : <[@count: index, @flag: index]>, index
// CHECK-NEXT:      %[[VAL_26:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_25]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_27:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_5]] : index, !felt.type
// CHECK-NEXT:      %[[VAL_28:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_26]], %[[VAL_27]]) : !felt.type, !felt.type
// CHECK-NEXT:      verif.ensure_compute %[[VAL_28]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_28]]
// CHECK-NEXT:    }
// CHECK-NEXT:  }
