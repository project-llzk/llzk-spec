// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/scoped-visibility.llzk 2>&1 | FileCheck %s
// END.

contract for A::Foo {
  ensure U == U;
  ensure other == other;
  ensure child.pub_out == child.pub_out;
}

// CHECK-DAG: unknown identifier `U`
// CHECK-DAG: unknown identifier `other`
// CHECK-DAG: unknown identifier `child.pub_out`
