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

predicate shl(x, y, z) = x << y == z

predicate shr(x, y, z) = x >> y == z

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
// CHECK-NEXT:    function.def @and(%[[VAL_6:[0-9a-zA-Z_\.]+]]: i1, %[[VAL_7:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_8:[0-9a-zA-Z_\.]+]] = bool.and %[[VAL_6]], %[[VAL_7]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_8]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @or(%[[VAL_9:[0-9a-zA-Z_\.]+]]: i1, %[[VAL_10:[0-9a-zA-Z_\.]+]]: i1) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_11:[0-9a-zA-Z_\.]+]] = bool.or %[[VAL_9]], %[[VAL_10]] : i1, i1
// CHECK-NEXT:      function.return %[[VAL_11]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @equal(%[[VAL_12:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_13:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_14:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_12]], %[[VAL_13]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_14]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @not_equal(%[[VAL_15:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_16:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_17:[0-9a-zA-Z_\.]+]] = bool.cmp ne(%[[VAL_15]], %[[VAL_16]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_17]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @less(%[[VAL_18:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_19:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_20:[0-9a-zA-Z_\.]+]] = bool.cmp lt(%[[VAL_18]], %[[VAL_19]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_20]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @less_than(%[[VAL_21:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_22:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_23:[0-9a-zA-Z_\.]+]] = bool.cmp le(%[[VAL_21]], %[[VAL_22]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_23]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @greater(%[[VAL_24:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_25:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_26:[0-9a-zA-Z_\.]+]] = bool.cmp gt(%[[VAL_24]], %[[VAL_25]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_26]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @greater_than(%[[VAL_27:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_28:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_29:[0-9a-zA-Z_\.]+]] = bool.cmp ge(%[[VAL_27]], %[[VAL_28]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_29]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @add(%[[VAL_30:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_31:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_32:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_33:[0-9a-zA-Z_\.]+]] = felt.add %[[VAL_30]], %[[VAL_31]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_34:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_33]], %[[VAL_32]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_34]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @sub(%[[VAL_35:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_36:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_37:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_38:[0-9a-zA-Z_\.]+]] = felt.sub %[[VAL_35]], %[[VAL_36]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_39:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_38]], %[[VAL_37]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_39]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @mul(%[[VAL_40:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_41:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_42:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_43:[0-9a-zA-Z_\.]+]] = felt.mul %[[VAL_40]], %[[VAL_41]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_44:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_43]], %[[VAL_42]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_44]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @div(%[[VAL_45:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_46:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_47:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_48:[0-9a-zA-Z_\.]+]] = felt.div %[[VAL_45]], %[[VAL_46]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_49:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_48]], %[[VAL_47]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_49]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @mod(%[[VAL_50:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_51:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_52:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_53:[0-9a-zA-Z_\.]+]] = felt.umod %[[VAL_50]], %[[VAL_51]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_54:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_53]], %[[VAL_52]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_54]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @bit_and(%[[VAL_55:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_56:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_57:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_58:[0-9a-zA-Z_\.]+]] = felt.bit_and %[[VAL_55]], %[[VAL_56]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_59:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_58]], %[[VAL_57]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_59]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @pow(%[[VAL_60:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_61:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_62:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_63:[0-9a-zA-Z_\.]+]] = felt.pow %[[VAL_60]], %[[VAL_61]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_64:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_63]], %[[VAL_62]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_64]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @shl(%[[VAL_65:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_66:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_67:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_68:[0-9a-zA-Z_\.]+]] = felt.shl %[[VAL_65]], %[[VAL_66]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_69:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_68]], %[[VAL_67]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_69]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:    function.def @shr(%[[VAL_70:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_71:[0-9a-zA-Z_\.]+]]: !felt.type, %[[VAL_72:[0-9a-zA-Z_\.]+]]: !felt.type) -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_73:[0-9a-zA-Z_\.]+]] = felt.shr %[[VAL_70]], %[[VAL_71]] : !felt.type, !felt.type
// CHECK-NEXT:      %[[VAL_74:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[VAL_73]], %[[VAL_72]]) : !felt.type, !felt.type
// CHECK-NEXT:      function.return %[[VAL_74]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
