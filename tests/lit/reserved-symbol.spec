// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/reserved-symbol.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/reserved-symbol.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for ReservedNames {
  ensure `return` == `return`;
}

// CHECK: "name": "return"
// CHECK-NOT: "`return`"
