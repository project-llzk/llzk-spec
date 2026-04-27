// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/poly-expr.llzk
// END.

contract for empty {
  ensure N == N;
}
