// REQUIRES: llzk-spec
// RUN: %llzk_spec %s %S/Inputs/poly-expr.llzk
// END.

contract for empty {
  ensure N == N;
}
