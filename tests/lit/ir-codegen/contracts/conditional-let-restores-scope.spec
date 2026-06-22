// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/../../Inputs/unnested.llzk --emit=ir | FileCheck %s
// END.

contract for Foo {
  let res = out == 0 ? out : out;
  ensure res == out;
}

// CHECK-LABEL: verif.contract @Foo$contract$0 for @Foo
// CHECK-NEXT:  %[[OUT:[0-9a-zA-Z_\.]+]] = struct.readm %arg0
// CHECK:       %[[RES:[0-9a-zA-Z_\.]+]] = scf.if
// CHECK:       %[[LHS:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[RES]]
// CHECK-NEXT:  %[[RHS:[0-9a-zA-Z_\.]+]] = cast.tofelt %[[OUT]]
// CHECK-NEXT:  %[[CMP:[0-9a-zA-Z_\.]+]] = bool.cmp eq(%[[LHS]], %[[RHS]])
// CHECK-NEXT:  verif.ensure_compute %[[CMP]]
// CHECK-NEXT:  verif.ensure_constrain %[[CMP]]
