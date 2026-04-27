// REQUIRES: llzk-spec
// RUN: not %llzk_spec %s %S/Inputs/valid.llzk 2>&1 | FileCheck %s
// END.

contract for Missing {
  ensure out == 0;
}

// CHECK: unknown contract target `Missing`
