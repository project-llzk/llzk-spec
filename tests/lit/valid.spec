// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  ensure out == 0;
}

// CHECK: "target"
// CHECK: "Foo"
