// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk 2>&1 | FileCheck %s
// END.

contract for Foo {
  predicate local(x) = x == out;
  ensure local;
}

// CHECK: unknown identifier `local`
