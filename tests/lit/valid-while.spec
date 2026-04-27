// REQUIRES: llzk-spec
// RUN: %llzk_spec %s %S/Inputs/valid-while.llzk
// END.

contract for Foo {
  ensure out == 0;
}
