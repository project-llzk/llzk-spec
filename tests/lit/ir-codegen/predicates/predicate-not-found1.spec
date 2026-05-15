// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  2>&1 | FileCheck %s
// END.

predicate foo(x) = bar(x)

// CHECK: predicate symbol 'bar' not found
