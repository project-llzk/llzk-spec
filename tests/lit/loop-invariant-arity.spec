// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk 2>&1 | FileCheck %s
// END.

contract for Foo {
  invariant for loop0(i) {
    ensure out == i;
  }
}

// CHECK: loop `loop0` expects 4 invariant bindings, found 1
