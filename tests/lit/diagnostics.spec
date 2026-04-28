// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/valid.llzk 2>&1 | FileCheck %s
// END.

predicate dup() = true
predicate dup() = true

contract for Foo {
  let x = nondet;
  let x = nondet;
  ensure missing == out;
  ensure missing_predicate();
  unused absent;
  return out;
}

// CHECK-DAG: duplicate predicate `dup`
// CHECK-DAG: duplicate local binding `x`
// CHECK-DAG: unknown identifier `missing`
// CHECK-DAG: unknown identifier `missing_predicate`
// CHECK-DAG: unused references unknown symbol `absent`
// CHECK-DAG: return is only valid inside predicates
