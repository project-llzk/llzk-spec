# `llzk-spec` Syntax

## Choosing A Contract Target

Contracts attach to LLZK symbols:

```spec
contract for Foo {
  ensure out == 0;
}
```

If the target struct or function is nested inside named modules, use its fully
qualified name:

```spec
contract for Bar::Foo {
  ensure out == out;
}
```

Current target rules:

- top-level `struct.def` and free `function.def` names are valid targets
- any target nested under named containers uses its fully qualified name with `::`
- named `poly.template` containers count here too, so template-defined structs are written as targets like `IsZero::IsZero`, `tmpl::empty`, or `OneHotTemplate::OneHot`
- there are no shorthand aliases for contract targets
- the verifier checks that the target exists in the LLZK IR

Example: `tests/lit/nested-modules.spec`

## How Names Resolve

Most spec names are LLZK symbol names, not source-language names.

### Root names

Plain `struct.member` roots are referenced directly by member name:

```spec
contract for Parent {
  ensure out == out;
}
```

Template params and `poly.expr` names that are visible from the target are also
referenced directly by name.

### Function inputs

`$arg[N]` always refers to the N-th contract input:

```spec
contract for IsZero::IsZero {
  ensure $arg[0] == 0 ? out == 1 : out == 0;
}
```

If the LLZK IR carries a `function.arg_name` attribute, that same input is also
available by its bare name:

```spec
contract for Num2Bits::Num2Bits {
  ensure in == $arg[0];
}
```

`$arg[N]` is zero-based (matches `@compute` and `@product` functions) from the spec author’s point of view.

`$res[N]` always refers to the N-th contract output when the target is a free
function:

```spec
contract for foo {
  ensure $res[0] == $res[0];
}
```

### Escaped identifiers

If an LLZK symbol name collides with a reserved spec keyword, escape it with
backticks:

```spec
contract for ReservedNames {
  ensure `return` == `return`;
}
```

Example: `tests/lit/reserved-symbol.spec`

## Statements

### `require`

Preconditions:

```spec
require n <= 252;
```

### `ensure`

Postconditions:

```spec
ensure out == 0 || out == 1;
```

### `let`

Local bindings:

```spec
let bit_i = ($arg[0] & 2 ** i) != 0 ? 1 : 0;
```

You can also bind `nondet`:

```spec
let x = nondet;
```

### `unused`

Marks a visible symbol as intentionally unused:

```spec
unused helper;
```

### `return`

Only valid inside block-bodied predicates:

```spec
predicate equals(a, b) {
  return a == b;
}
```

## Predicates

Predicates may be top-level:

```spec
predicate always() = true
```

Or nested inside a contract:

```spec
contract for Foo {
  predicate local(x) = x == out
  ensure local(out);
}
```

Locally defined predicates capture the surrounding contract context. In the
example above, `local` can reference `out` because it is visible from the
enclosing contract target.

Supported forms:

- expression-bodied: `predicate p(x) = expr`
- block-bodied: `predicate p(x) { ... return expr; }`

Example: `tests/lit/predicates.spec`

## Scopes

Statements or blocks can be restricted to witness/compute or constrain logic:

```spec
compute ensure out == out;

constrain {
  require out == out;
}
```

`witness` is syntactic sugar for `compute`.

Example: `tests/lit/scopes.spec`

## Expressions

### Literals and operators

The current language supports:

- booleans: `true`, `false`
- numbers (field elements)
- unary: `!`, `-`
- logical: `&&`, `||`
- equality: `==`, `!=`
- relational: `<`, `<=`, `>`, `>=`
- arithmetic: `+`, `-`, `*`, `/`, `%`, `**`
- bitwise-and: `&`
- ternary conditionals: `cond ? a : b`

Expressions are composable:

```spec
ensure d - ((a + b) * c) == 0;
```

Examples: `tests/lit/expressions.spec`, `tests/lit/parentheses.spec`

### Calls

Predicate calls use normal call syntax:

```spec
ensure equals(out, out);
```

### Arrays

Arrays use `[]` indexing:

```spec
ensure out[i] == 0;
ensure arr[0] == arr[1];
```

`len(expr)` is supported:

```spec
ensure len(bits) == N;
```

### Member access

Nested members use dot notation.

#### Nested `struct.type` members

For a `struct.member` whose type is `struct.type`, nested fields are written with
`.`:

```spec
ensure child.pub_out == child.pub_out;
```

Only nested members marked `llzk.pub` are accessible. Private nested members are
rejected:

```spec
ensure child.secret == 0; // invalid if `secret` is not public
```

#### Nested `pod.type` fields

`pod.type` fields also use dot notation:

```spec
ensure pod.count == pod.flag;
```

These do not require `llzk.pub`.

#### Arrays of components or arrays inside components

Combine `[]` and `.` normally:

```spec
ensure children[0].pub_out == children[1].pub_out;
ensure child.arr_pub[0] == child.arr_pub[0];
```

Example source of truth: `tests/lit/member-access.spec`

### Quantifiers

Universal and existential quantifiers are expressions:

```spec
ensure forall i in 0..len(out), out[i] == out[i];
ensure exists b in bits, b == 1;
ensure forall value in out, value == value;
```

Domains may be:

- ranges: `0..n`, `(i + 1)..N`
- general expressions such as arrays

Example source of truth: `tests/lit/quantifiers.spec`

## Loop Invariants

Loop invariants attach to LLZK `scf.for` and `scf.while` loops:

```spec
invariant for loop0(lb, i, ub, stride) {
  decreases ub - i;
  step i == old(i) + stride;
  ensure i <= ub;
}
```

### Loop names

- if the IR loop has a string `loop_label` attribute, use that name
- otherwise the compiler generates `loop0`, `loop1`, and so on

Generated names are scoped:

- to the containing `struct.def` for struct compute/constrain/product loops
- to the containing free `function.def` for loops outside structs
- loops inside `poly.expr` do not receive generated names

### Binding order

For `scf.for`, bindings are:

1. lower bound
2. induction variable
3. upper bound
4. step
5. iter args

For `scf.while`, bindings are the loop-carried block arguments in order.

The verifier checks the binding count exactly.

### Invariant-only statements

Inside invariant bodies you may use:

- `increases expr;`
- `decreases expr;`
- `step expr;`

`old(expr)` is only valid inside a `step` expression.

Examples:

- `tests/lit/loop-invariant-for.spec`
- `tests/lit/loop-invariant-while.spec`
- `tests/lit/loop-invariant-labeled-for.spec`

## Current Limitations

The current implementation is intentionally structural.

- It validates symbol existence and visibility, not full semantic correctness.
- It does not yet lower specs into MLIR (dependent on `verif` dialect implementation).
- `$arg[N]` always addresses contract inputs positionally.
- `$res[N]` always addresses contract outputs positionally.
- Bare input names are additionally available when the LLZK IR carries a
  `function.arg_name` attribute.
- Nested `struct.type` access checks public visibility, but this is still name-
  and shape-based validation rather than deep type reasoning.
- Diagnostics and examples should be treated as the source of truth over any
  unstated syntax assumptions.

## Where To Look Next

- End-to-end spec examples: `tests/lit/*.spec`
- LLZK examples: `tests/lit/Inputs/*.llzk`
- Parser grammar: `src/grammar/llzk_spec.pest`
