# llzk-spec

`llzk-spec` is a Rust compiler for the LLZK specification language. Phase 1 parses
`llzk-spec` source files, validates them against an LLZK IR module, and can emit the
parsed AST for debugging or test assertions.

## Usage

```sh
llzk-spec path/to/spec.llzk-spec path/to/module.mlir
llzk-spec path/to/spec.llzk-spec path/to/module.mlir --emit-ast - --format json
```

## Development

Use the provided Nix shell:

```sh
nix develop
cargo test
LLZK_SPEC_BIN=target/debug/llzk-spec lit -a tests/lit
```
