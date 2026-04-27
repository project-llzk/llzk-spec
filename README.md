# llzk-spec

`llzk-spec` is a Rust compiler for the LLZK specification language. Phase 1 parses
`llzk-spec` source files, validates them against an LLZK IR module, and can emit the
parsed AST for debugging or test assertions.

## Current Features

- Parses `.spec` files into the llzk-spec AST.
- Loads LLZK IR and extracts symbol, visible name, and loop-label metadata.
- Verifies contract targets, referenced symbols, and loop labels against LLZK IR metadata.
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

## Development

Use the provided Nix shell:

```sh
nix develop
cargo test
```

Lit-style end-to-end tests live under `tests/lit` and are run by `cargo test`.
The Nix shell provides `FileCheck` for those tests.

## Repository Layout

- `src/grammar`: [pest](https://github.com/pest-parser/pest) grammar for the `llzk-spec` language.
- `src`: parser, AST, diagnostics, IR loading, verification, and CLI code.
- `tests/lit`: cargo-integrated lit-style end-to-end tests.
- `tests/lit/Inputs`: LLZK IR inputs used by the lit-style tests.
