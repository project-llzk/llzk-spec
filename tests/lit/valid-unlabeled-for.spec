// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Foo {
  ensure out == 0;
  invariant for loop0 (lb, i, ub, step) {
    increases i;
  }
}

// CHECK: "target"
// CHECK: "Foo"
