// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.

predicate never() = false

// CHECK-LABEL: module attributes {llzk.lang} {
// CHECK-NEXT:    function.def @never() -> i1 attributes {function.allow_non_native_field_ops} {
// CHECK-NEXT:      %[[VAL_0:[0-9a-zA-Z_\.]+]] = arith.constant false
// CHECK-NEXT:      function.return %[[VAL_0]] : i1
// CHECK-NEXT:    }
// CHECK-NEXT:  }
