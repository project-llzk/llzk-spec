# `llzk-spec` Design

`llzk-spec` is a specification language for LLZK IR.
It gives authors a way to describe requirements, postconditions, predicates, and loop invariants for
circuits that have already been compiled to LLZK.

This document explains the language semantics at a high level.
For concrete syntax and examples, see [SYNTAX.md](SYNTAX.md).

## Design Goals

`llzk-spec` is designed to specify compiled LLZK IR, rather than source ZK DSLs.
We chose to implement `llzk-spec` in this way for the following reasons:

- Specification syntax can be uniform regardless of source-language idiosyncrasies.
- Circuits written in languages without native specifications can have specifications written for them.

### Current Limitations

- The compiler currently only performs structural verification (i.e., checking that referenced names,
  contract targets, members, predicates, and loops exist and are visible)
- Lowering to (currently yet-to-be-implemented) `verif` dialect is unsupported (see prior parenthetical).
- All values are assumed to be field elements; reasoning over specific-width integer logic is future work.

## Contracts

A contract is the top-level specification for an LLZK symbol:

```spec
contract for IsZero::IsZero {
  ensure out == 0 || out == 1;
  ensure arg[0] == 0 ? out == 1 : out == 0;
}
```

The target after `contract for` must resolve to a symbol in the LLZK IR. For
template-generated or nested IR, authors use fully qualified LLZK names such as
`IsZero::IsZero`, `Num2Bits::Num2Bits`, or `tmpl::empty`.

Inside a contract, bare names are resolved against the symbols visible from that
target. This includes public members, visible template parameters, visible
`poly.expr` names, and local bindings introduced by the spec.

## Requirements And Ensures

`require` and `ensure` are the core assertion forms:

```spec
contract for LessThan::LessThan {
  require n <= 252;
  ensure out == 0 || out == 1;
  ensure out == 1 ? arg[0][0] < arg[0][1] : arg[0][0] >= arg[0][1];
}
```

`require` expresses a precondition that the spec assumes. `ensure` expresses a
postcondition that should hold for the selected contract scope.

## Compute And Constrain Scopes

Contracts can distinguish witness computation from constraint generation:

```spec
contract for Foo {
  compute ensure /* expression */;

  constrain {
    require /* expression */;
    ensure /* expression */;
  }
}
```

`compute` and `witness` refer to witness generation logic.
`constrain` refers to constraint generation logic.
Unscoped statements apply to the whole contract (i.e., to compute and constrain logic).


## Predicates

Predicates name reusable boolean expressions:

```spec
predicate is_bool(x) = x == 0 || x == 1

contract for Num2Bits::Num2Bits {
  ensure forall i in 0..n, is_bool(out[i]);
}
```

Predicates may be top level or nested inside contracts:

```spec
contract for Foo {
  predicate equals_out(x) {
    return x == out;
  }

  ensure equals_out(out);
}
```

Nested predicates can capture names from their surrounding contract context. Predicate
parameters and local `let` bindings shadow outer values only within their lexical
scope. Predicate names are callable with normal call syntax (i.e., `predicate(/* args */)`); a bare predicate name is
not a value expression.

## Local Bindings And Nondeterminism

`let` introduces a spec-local value:

```spec
contract for Foo {
  let bit_i = (arg[0] & (2 ** i)) != 0 ? 1 : 0;
  ensure bit_i == out[i];
}
```

`let x = nondet;` introduces an unconstrained value in the spec. This can useful for
expressing existential or helper values before the language has richer type and
lowering support:

// TODO(Codex): come up with a better example
```spec
contract for Foo {
  let witness = nondet;
  ensure witness * witness == out;
}
```

## Name Resolution

Names resolve from most local to least local:

1. local bindings and predicate parameters
2. predicates defined in the current lexical scope
3. top-level predicates, when called as predicates
4. symbols visible from the active contract target

LLZK symbols are the source of truth. If a source language renames or specializes
constructs during compilation, the spec must use the LLZK-visible name.

Unnamed function arguments can be referenced with `arg[N]`:

```spec
contract for IsZero::IsZero {
  ensure arg[0] == 0 ? out == 1 : out == 0;
}
```

`arg[N]` is a workaround since LLZK currently does not assign symbol names to function arguments,
but will be addressed in a future LLZK release.

## Members And Visibility

Member access uses dot notation:

```spec
contract for Parent {
  ensure child.pub_out == child.pub_out;
  ensure children[0].pub_out == children[1].pub_out;
}
```

For nested `struct.type` members, only public members are spec-visible. Accessing a
private nested member is rejected:

```spec
contract for Parent {
  ensure child.secret == 0; // rejected if `secret` is not public
}
```

`pod.type` fields are treated as structural fields and can be accessed by name.

## Quantifiers

Quantifiers are expressions:

```spec
contract for OneHotTemplate::OneHot {
  ensure forall i in 0..N, i == arg[0] ? bits[i] == 1 : bits[i] == 0;
  ensure exists bit in bits, bit == 1;
}
```

Range domains such as `0..N` bind the variable over an integer-like interval.
Expression domains such as `bits` bind the variable over the values in that
expression.

## Loop Invariants

Loop invariants attach specs to LLZK `scf.for` and `scf.while` loops:

```spec
contract for Num2Bits::Num2Bits {
  invariant for loop1(e2, i, lc1) {
    decreases n - i;
    step lc1 == old(lc1) + out[i] * e2;
    ensure out[i] == 0 || out[i] == 1;
  }
}
```

A loop name is either:

- the loop's string `loop_label` attribute, when present
- a generated name such as `loop0`, assigned from IR walk order in the relevant
  struct or function scope

Invariant bindings expose the loop values to the invariant body. For `scf.for`, the
binding order is lower bound, induction variable, upper bound, step, then iter args.
For `scf.while`, bindings are the loop-carried block arguments in order.

The verifier checks that the loop exists and that the invariant supplies exactly the
expected number of bindings.

Invariant bodies can contain:

```spec
decreases n - i;
step i == old(i) + 1;
ensure i <= n;
```

`increases`, `decreases`, and `step` are only valid inside invariants. `old(expr)` is
only valid inside a `step` expression and denotes the previous iteration's value for
future lowering.


## Running `llzk-spec`

Use the CLI with both a spec file and an LLZK IR file:

```sh
llzk-spec --spec path/to/spec.spec --llzk path/to/module.llzk
llzk-spec --spec path/to/spec.spec --llzk path/to/module.llzk --emit-ast - --format json
```

Or, during development, you can also just use `cargo run`:

```sh
cargo run -- --spec path/to/spec.spec --llzk path/to/module.llzk
cargo run -- --spec path/to/spec.spec --llzk path/to/module.llzk --emit-ast - --format json
```


In this repository, the normal development environment is:

```sh
nix develop
cargo test
```

The end-to-end language examples live in `tests/lit/*.spec`.
