// REQUIRES: llzk-spec
// RUN: %llzk_spec %s %S/Inputs/poly-param.llzk
// END.

contract for empty {
  ensure T == T;
}
