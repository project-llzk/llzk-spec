// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-while.llzk
// END.

contract for Foo {
  ensure out == 0;
}
