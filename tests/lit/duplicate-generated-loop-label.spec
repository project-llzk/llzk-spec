// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/duplicate-generated-loop-label.llzk 2>&1 | FileCheck %s
// END.

contract for Foo {
  ensure out == out;
}

// CHECK: duplicate loop name `loop0`
