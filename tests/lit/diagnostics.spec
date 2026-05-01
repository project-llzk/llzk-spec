// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk 2>&1 | FileCheck %s
// END.

predicate dup() = true
predicate dup() = true
predicate noop() = true

contract for Foo {
  let x = nondet;
  let x = nondet;
  ensure missing == out;
  ensure missing_predicate();
  ensure old(out) == out;
  increases out;
  decreases out;
  step out == out;
  unused absent;
  return out;
  ensure x(6);
  // This is the invalid expression
  ensure noop().made_up_member == 0;
}

// CHECK-DAG: duplicate predicate `dup`
// CHECK-DAG: duplicate local binding `x`
// CHECK-DAG: unknown identifier `missing`
// CHECK-DAG: unknown predicate `missing_predicate`
// CHECK-DAG: old is only valid inside step expressions
// CHECK-DAG: increases is only valid inside invariants
// CHECK-DAG: decreases is only valid inside invariants
// CHECK-DAG: step is only valid inside invariants
// CHECK-DAG: unused references unknown symbol `absent`
// CHECK-DAG: return is only valid inside predicates
// CHECK-DAG: unknown predicate `x`
// CHECK-DAG: invalid expression
