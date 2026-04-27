// REQUIRES: llzk-spec
// RUN: not %llzk_spec %s %S/Inputs/valid.llzk 2>&1 | FileCheck %s
// END.

contract for Foo {
  ensure ;
}

// CHECK: syntax error
