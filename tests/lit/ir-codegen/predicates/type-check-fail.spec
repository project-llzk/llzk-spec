// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/../../Inputs/valid-unlabeled-for.llzk --emit=ir  2>&1 | FileCheck %s
// END.


predicate unused_param(x) = true

// CHECK: parameter 'x' in predicate 'unused_param' has an ambigous type
