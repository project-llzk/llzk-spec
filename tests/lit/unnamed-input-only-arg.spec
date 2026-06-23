// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/product-scoped-loops.llzk 2>&1 | FileCheck %s
// END.

contract for tmpl::Prod {
  ensure arg[0] == arg[0];
  ensure input == input;
}

// CHECK: unknown identifier `input`
