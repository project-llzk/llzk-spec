// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/doc-example-one-hot.llzk --emit=ast --emit-format json | FileCheck %s
// END.

contract for OneHotTemplate::OneHot {
  ensure len(bits) == N && arg[0] <= N;
  ensure forall i in 0..N, i == arg[0] ? bits[i] == 1 : bits[i] == 0;
}

contract for OneHotTemplate::OneHot {
  ensure len(bits) == N && arg[0] <= N;
  ensure forall b in bits, b == 0 || b == 1;
  ensure exists b in bits, b == 1;
  ensure forall i in 0..(N-1), forall j in (i+1)..N, (bits[i] * bits[j]) == 0;
  ensure bits[arg[0]] == 1;
}

predicate all_bits_boolean(bit_arr) {
  return forall i in 0..len(bit_arr), bit_arr[i] == 0 || bit_arr[i] == 1;
}

contract for OneHotTemplate::OneHot {
  ensure len(bits) == N && arg[0] <= N;
  ensure all_bits_boolean(bits);
  ensure exists b in bits, b == 1;
  ensure forall i in 0..(N-1), forall j in (i+1)..N, (bits[i] * bits[j]) == 0;
  ensure bits[arg[0]] == 1;
}

// CHECK-DAG: "name": "OneHotTemplate::OneHot"
// CHECK-DAG: "name": "all_bits_boolean"
// CHECK-DAG: "kind": "len"
// CHECK-DAG: "quantifier_kind": "forall"
// CHECK-DAG: "quantifier_kind": "exists"
// CHECK-DAG: "kind": "conditional"
// CHECK-DAG: "kind": "call"
// CHECK-DAG: "kind": "arg"
// CHECK-DAG: "index": 0
// CHECK-DAG: "kind": "index"
