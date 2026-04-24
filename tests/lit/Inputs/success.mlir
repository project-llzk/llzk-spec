builtin.module {
  poly.template @Foo {
    struct.def @Foo {
      struct.member @out : !felt.type<"bn128"> {llzk.pub}
      function.def @compute(%arg0: !felt.type<"bn128">) -> !struct.type<@Foo::@Foo<[]>> attributes {function.allow_witness} {
        function.return
      }
      function.def @constrain(%arg0: !struct.type<@Foo::@Foo<[]>>, %arg1: !felt.type<"bn128">) attributes {function.allow_constraint} {
        function.return
      }
    }
  }
}
