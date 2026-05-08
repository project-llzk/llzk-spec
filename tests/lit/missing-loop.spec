// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk 2>&1 | FileCheck %s
// END.

contract for Foo {
  invariant for loop2(lb, i, ub, step) {
    ensure out == i;
  }
}

// CHECK: unknown loop `loop2`
