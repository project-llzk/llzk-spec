// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate felts_and_bool(f1, f2, b1) = ((f1 + f2) == 0) && b1


// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @always() -> i1 {
// CHECK-NEXT:      %[[VAL_0:[0-9a-zA-Z_\.]+]] = arith.constant true
// CHECK-NEXT:      function.return %[[VAL_0]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
