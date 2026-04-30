// REQUIRES: llzk-spec
// RUN: not %llzk_spec --spec %s --llzk %S/Inputs/member-access.llzk 2>&1 | FileCheck %s
// END.

contract for Parent {
  ensure children[0].secret == 0;
}

// CHECK: member `children[].secret` is not public
