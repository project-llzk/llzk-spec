// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/member-access.llzk 2>&1 | FileCheck %s
// END.

contract for Parent {
  ensure child.secret == 0;
}

// CHECK: member `child.secret` is not public
