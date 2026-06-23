// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/one-hot.llzk --emit=ir | FileCheck %s
// END.

contract for OneHotTemplate::OneHot {
  ensure forall i in 0..N, i == arg[0] ? bits[i] == 1 : bits[i] == 0;
  ensure exists bit in bits, bit == 1;
}

// CHECK: #[[$ATTR_0:[0-9a-zA-Z_\.]+]] = affine_map<()[s0, s1] -> (s1 - s0)>

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    verif.contract @OneHotTemplate$OneHot$contract$0 for @OneHotTemplate::@OneHot (%[[VAL_0:[0-9a-zA-Z_\.]+]]: !struct.type<@OneHotTemplate::@OneHot<[@N]>>, %[[VAL_1:[0-9a-zA-Z_\.]+]]: !felt.type) {
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = poly.read_const @N : index
// CHECK-NEXT:      %[[VAL_3:[0-9a-zA-Z_\.]+]] = struct.readm %[[VAL_0]][@bits] : <@OneHotTemplate::@OneHot<[@N]>>, !array.type<@N x !felt.type>
// CHECK-NEXT:      %[[VAL_4:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = cast.toindex %[[VAL_4]] : !felt.type
// CHECK-NEXT:      %[[VAL_6:[0-9a-zA-Z_\.]+]] = array.new{(){{\[}}%[[VAL_5]], %[[VAL_2]]]} : <#[[$ATTR_0]] x !felt.type>
// CHECK-NEXT:      %[[VAL_7:[0-9a-zA-Z_\.]+]] = arith.constant 1 : index
// CHECK-NEXT:      scf.for %[[VAL_8:[0-9a-zA-Z_\.]+]] = %[[VAL_5]] to %[[VAL_2]] step %[[VAL_7]] {
// CHECK-NEXT:        %[[VAL_9:[0-9a-zA-Z_\.]+]] = arith.subi %[[VAL_8]], %[[VAL_5]] : index
// CHECK-NEXT:        %[[VAL_10:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[VAL_8]] : index, !felt.type
// CHECK-NEXT:        array.write %[[VAL_6]]{{\[}}%[[VAL_9]]] = %[[VAL_10]] : <#[[$ATTR_0]] x !felt.type>, !felt.type
// CHECK-NEXT:      }
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = bool.forall %[[VAL_12:[0-9a-zA-Z_\.]+]]: !felt.type in %[[VAL_6]] : !array.type<#[[$ATTR_0]] x !felt.type> {
// CHECK-NEXT:        %[[VAL_13:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_12]], %[[VAL_1]]) : !felt.type, !felt.type
// CHECK-NEXT:        %[[VAL_14:[0-9a-zA-Z_\.]+]] = scf.if %[[VAL_13]] -> (i1) {
// CHECK-NEXT:          %[[VAL_15:[0-9a-zA-Z_\.]+]] = cast.toindex %[[VAL_12]] : !felt.type
// CHECK-NEXT:          %[[VAL_16:[0-9a-zA-Z_\.]+]] = array.read %[[VAL_3]]{{\[}}%[[VAL_15]]] : <@N x !felt.type>, !felt.type
// CHECK-NEXT:          %[[VAL_17:[0-9a-zA-Z_\.]+]] = felt.const  1
// CHECK-NEXT:          %[[VAL_18:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_16]], %[[VAL_17]]) : !felt.type, !felt.type
// CHECK-NEXT:          scf.yield %[[VAL_18]] : i1
// CHECK-NEXT:        } else {
// CHECK-NEXT:          %[[VAL_19:[0-9a-zA-Z_\.]+]] = cast.toindex %[[VAL_12]] : !felt.type
// CHECK-NEXT:          %[[VAL_20:[0-9a-zA-Z_\.]+]] = array.read %[[VAL_3]]{{\[}}%[[VAL_19]]] : <@N x !felt.type>, !felt.type
// CHECK-NEXT:          %[[VAL_21:[0-9a-zA-Z_\.]+]] = felt.const  0
// CHECK-NEXT:          %[[VAL_22:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_20]], %[[VAL_21]]) : !felt.type, !felt.type
// CHECK-NEXT:          scf.yield %[[VAL_22]] : i1
// CHECK-NEXT:        }
// CHECK-NEXT:        bool.yield %[[VAL_14]]
// CHECK-NEXT:      }
// CHECK-NEXT:      verif.ensure_compute %[[VAL_11]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_11]]
// CHECK-NEXT:      %[[VAL_23:[0-9a-zA-Z_\.]+]] = bool.exists %[[VAL_24:[0-9a-zA-Z_\.]+]]: !felt.type in %[[VAL_3]] : !array.type<@N x !felt.type> {
// CHECK-NEXT:        %[[VAL_25:[0-9a-zA-Z_\.]+]] = felt.const  1
// CHECK-NEXT:        %[[VAL_26:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_24]], %[[VAL_25]]) : !felt.type, !felt.type
// CHECK-NEXT:        bool.yield %[[VAL_26]]
// CHECK-NEXT:      }
// CHECK-NEXT:      verif.ensure_compute %[[VAL_23]]
// CHECK-NEXT:      verif.ensure_constrain %[[VAL_23]]
// CHECK-NEXT:    }
// CHECK-NEXT:    poly.template @OneHotTemplate {
// CHECK-NEXT:      poly.param @N : index
// CHECK-NEXT:      struct.def @OneHot {
// CHECK-NEXT:        struct.member @bits : !array.type<@N x !felt.type>
// CHECK-NEXT:        function.def @compute(%[[VAL_27:[0-9a-zA-Z_\.]+]]: !felt.type) -> !struct.type<@OneHotTemplate::@OneHot<[@N]>> attributes {function.allow_witness} {
// CHECK-NEXT:          %[[VAL_28:[0-9a-zA-Z_\.]+]] = struct.new : <@OneHotTemplate::@OneHot<[@N]>>
// CHECK-NEXT:          function.return %[[VAL_28]] : !struct.type<@OneHotTemplate::@OneHot<[@N]>>
// CHECK-NEXT:        }
// CHECK-NEXT:        function.def @constrain(%[[VAL_29:[0-9a-zA-Z_\.]+]]: !struct.type<@OneHotTemplate::@OneHot<[@N]>>, %[[VAL_30:[0-9a-zA-Z_\.]+]]: !felt.type) attributes {function.allow_constraint} {
// CHECK-NEXT:          function.return
// CHECK-NEXT:        }
// CHECK-NEXT:      }
// CHECK-NEXT:    }
// CHECK-NEXT:  }
