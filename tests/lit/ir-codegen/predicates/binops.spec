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
// CHECK-NEXT:    function.def @always() -> i1 {
// CHECK-NEXT:      %[[VAL_0:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:      function.return %[[VAL_0]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
