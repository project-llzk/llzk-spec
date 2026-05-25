// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/nested-modules.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for Bar::Foo {
  invariant for same(lb, i, ub, stride) {
    ensure out == out;
  }
}

// CHECK-LABEL: module attributes {llzk.lang} {
