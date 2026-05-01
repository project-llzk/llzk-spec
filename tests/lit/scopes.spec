// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/valid-unlabeled-for.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Foo {
  compute ensure out == out;

  witness {
    let w = nondet;
    ensure w == w;
  }

  constrain {
    require out == out;
  }

  {
    let nested = nondet;
    ensure nested == nested;
  }
}

// CHECK-DAG: "kind": "scoped"
// CHECK-DAG: "scope": "compute"
// CHECK-DAG: "scope": "constrain"
// CHECK-DAG: "kind": "block"
