// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/doc-example-is-zero.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for IsZero {
  ensure out == 0 || out == 1;
}

contract for IsZero {
  ensure arg[0] == 0 ? out == 1 : out == 0;
}

// CHECK-DAG: "name": "IsZero"
// CHECK-DAG: "op": "or"
// CHECK-DAG: "kind": "conditional"
// CHECK-DAG: "kind": "arg"
// CHECK-DAG: "index": 0
// CHECK-DAG: "name": "out"
