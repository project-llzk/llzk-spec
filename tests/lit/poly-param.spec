// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/poly-param.llzk
// END.

contract for tmpl::empty {
  ensure T == T;
}
