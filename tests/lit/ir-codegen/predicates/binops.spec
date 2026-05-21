// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate and(x, y) = x && y

predicate or(x, y) = x || y 

predicate equal(x, y) = x == y 

predicate not_equal(x, y) = x != y 

predicate less(x, y) = x < y 

predicate less_than(x, y) = x <= y 

predicate greater(x, y) = x > y 

predicate greater_than(x, y) = x >= y 

predicate add(x, y, z) = x + y == z

predicate sub(x, y, z) = x - y == z

predicate mul(x, y, z) = x * y == z

predicate div(x, y, z) = x / y == z

predicate mod(x, y, z) = x % y == z

predicate bit_and(x, y, z) = x & y == z

predicate pow(x, y, z) = x ** y == z

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @and(%[[VAL_0:[0-9a-zA-Z_\.]+]]: i1, %[[VAL_1:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_2:[0-9a-zA-Z_\.]+]] = bool.and %[[VAL_0]], %[[VAL_1]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_2]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @or(%[[VAL_3:[0-9a-zA-Z_\.]+]]: i1, %[[VAL_4:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_5:[0-9a-zA-Z_\.]+]] = bool.or %[[VAL_3]], %[[VAL_4]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_5]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @equal(%[[VAL_6:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_7:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_6]], %[[VAL_7]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_8]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @not_equal(%[[VAL_9:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_10:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = bool.cmp ne(%[[VAL_9]], %[[VAL_10]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_11]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @less(%[[VAL_12:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_13:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_14:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_12]], %[[VAL_13]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_14]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @less_than(%[[VAL_15:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_16:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_17:[0-9a-zA-Z_\.]+]] = bool.cmp le(%[[VAL_15]], %[[VAL_16]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_17]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @greater(%[[VAL_18:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_19:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_20:[0-9a-zA-Z_\.]+]] = bool.cmp gt(%[[VAL_18]], %[[VAL_19]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_20]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @greater_than(%[[VAL_21:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_22:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_23:[0-9a-zA-Z_\.]+]] = bool.cmp ge(%[[VAL_21]], %[[VAL_22]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_23]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @add(%[[VAL_24:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_25:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_26:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_27:[0-9a-zA-Z_\.]+]] = felt.add %[[VAL_24]], %[[VAL_25]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_28:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_27]], %[[VAL_26]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_28]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @sub(%[[VAL_29:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_30:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_31:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_32:[0-9a-zA-Z_\.]+]] = felt.sub %[[VAL_29]], %[[VAL_30]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_33:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_32]], %[[VAL_31]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_33]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @mul(%[[VAL_34:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_35:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_36:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_37:[0-9a-zA-Z_\.]+]] = felt.mul %[[VAL_34]], %[[VAL_35]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_38:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_37]], %[[VAL_36]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_38]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @div(%[[VAL_39:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_40:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_41:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_42:[0-9a-zA-Z_\.]+]] = felt.div %[[VAL_39]], %[[VAL_40]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_43:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_42]], %[[VAL_41]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_43]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @mod(%[[VAL_44:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_45:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_46:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_47:[0-9a-zA-Z_\.]+]] = felt.umod %[[VAL_44]], %[[VAL_45]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_48:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_47]], %[[VAL_46]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_48]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @bit_and(%[[VAL_49:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_50:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_51:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_52:[0-9a-zA-Z_\.]+]] = felt.bit_and %[[VAL_49]], %[[VAL_50]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_53:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_52]], %[[VAL_51]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_53]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @pow(%[[VAL_54:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_55:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_56:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_57:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_54]], %[[VAL_55]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_58:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_57]], %[[VAL_56]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_58]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
