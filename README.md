# llzk-spec

`llzk-spec` is a Rust compiler for the LLZK specification language. Phase 1 parses
`llzk-spec` source files, validates them against an LLZK IR module, and can emit the
parsed AST for debugging or test assertions.

## Usage

```sh
llzk-spec --spec path/to/spec.llzk-spec --llzk path/to/module.mlir
llzk-spec --spec path/to/spec.llzk-spec --llzk path/to/module.mlir --emit-ast - --format json
```

## Development

Use the provided Nix shell:

```sh
nix develop
cargo test
```

Lit-style end-to-end tests live under `tests/lit` and are run by `cargo test`.
The Nix shell provides `FileCheck` for those tests.

## Repository Layout

- `src/grammar`: pest grammar for the `llzk-spec` language.
- `src`: parser, AST, diagnostics, IR loading, verification, and CLI code.
- `tests/lit`: cargo-integrated lit-style end-to-end tests.
- `tests/lit/Inputs`: LLZK IR inputs used by the lit-style tests.
