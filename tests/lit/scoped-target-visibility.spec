// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/scoped-visibility.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for A::Foo {
  ensure out == 0;

  // Captures context
  predicate foo(i) = out == i;
  ensure foo(out);
}

// CHECK-DAG: "target"
// CHECK-DAG: "A::Foo"
// CHECK-DAG: "name": "foo"
// CHECK-DAG: "kind": "call"
