// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  | FileCheck %s
// END.
// Emitting IR for predicates within predicates is not implemented yet.
// XFAIL: *

// Different arity that inner foo for catching errors more easily
predicate foo(x, y) = !x || y

predicate bar(x) {
  predicate foo(y) = !y

  return foo(!x);
}
