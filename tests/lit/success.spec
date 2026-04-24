// RUN: %llzk_spec %s %S/Inputs/success.mlir --emit-ast - --format json | FileCheck %s
// CHECK: "target": {
// CHECK: "name": "Foo"

contract for Foo {
  ensure out == 0;
}
