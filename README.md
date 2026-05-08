# llzk-spec

`llzk-spec` is a Rust compiler for the LLZK specification language. Phase 1 parses
`llzk-spec` source files, validates them against an LLZK IR module, and can emit the
parsed AST for debugging or test assertions.

## Current Features

- Parses `.spec` files into the llzk-spec AST.
- Loads LLZK IR and extracts symbol, visible name, and loop metadata.
- Verifies contract targets, referenced symbols, and loop invariant bindings against LLZK IR metadata.
- Emits the parsed AST in debug or JSON format.
- Runs cargo-integrated lit-style end-to-end tests.

## Roadmap

- Lower llzk-spec AST into the future `verif` MLIR dialect.
- Expand type checking beyond phase-1 symbol and loop-label validation.
- Add richer diagnostics as the language and lowering pipeline grow.

## Usage

```sh
llzk-spec --spec path/to/spec.llzk-spec --llzk path/to/module.mlir
llzk-spec --spec path/to/spec.llzk-spec --llzk path/to/module.mlir --emit-ast - --format json
```

## Documentation

The high-level semantics and examples are described in [docs/DESIGN.md](docs/DESIGN.md).
The concrete syntax reference is [docs/SYNTAX.md](docs/SYNTAX.md).

## Development

Use the provided Nix shell:

```sh
nix develop
cargo test
```

Lit-style end-to-end tests live under `tests/lit` and are run by `cargo test`.
The Nix shell provides `FileCheck` for those tests.

## Loop Invariants

Loop invariants bind the SSA values for an LLZK `scf.for` or `scf.while` loop:

```spec
invariant for loop0(lb, i, ub, stride) {
  decreases ub - i;
  step i == old(i) + stride;
  ensure i <= ub;
}
```

If a loop has a string `loop_label` attribute, that label is the spec-visible
name. Unlabeled loops use generated zero-indexed names such as `loop0` and
`loop1`, assigned in IR walk order within each `struct.def`, or within a free
`function.def` when the loop is not inside a struct. Loops inside `poly.expr`
do not receive generated spec names. `scf.for` bindings are lower bound,
induction variable, upper bound, step, then iter args. `scf.while` bindings are
the loop-carried block arguments in order.

`increases`, `decreases`, and `step` statements are valid inside invariants.
`old(expr)` is valid only inside a `step` expression and refers to the previous
iteration's value.

## Repository Layout

- `src/grammar`: [pest](https://github.com/pest-parser/pest) grammar for the `llzk-spec` language.
- `src`: parser, AST, diagnostics, IR loading, verification, and CLI code.
- `tests/lit`: cargo-integrated lit-style end-to-end tests.
- `tests/lit/Inputs`: LLZK IR inputs used by the lit-style tests.
