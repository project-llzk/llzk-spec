// REQUIRES: llzk-spec
// RUN: %llzk_spec --spec %s --llzk %S/Inputs/member-access.llzk --emit-ast - --format json | FileCheck %s
// END.

contract for Parent {
  ensure child.pub_out == child.pub_out;
  ensure children[0].pub_out == children[1].pub_out;
  ensure child.arr_pub[0] == child.arr_pub[0];
  ensure pod.count == pod.flag ? arr[0] == arr[1] : arr[1] == arr[0];
}

// CHECK-DAG: "kind": "member"
// CHECK-DAG: "name": "child"
// CHECK-DAG: "name": "pub_out"
// CHECK-DAG: "name": "arr_pub"
// CHECK-DAG: "name": "children"
// CHECK-DAG: "name": "pod"
// CHECK-DAG: "name": "count"
// CHECK-DAG: "name": "flag"
