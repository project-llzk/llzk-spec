# llzk-spec Agent Guide

This repository contains the Phase 1 `llzk-spec` compiler: a Rust CLI that parses
high-level `.spec` files, loads an LLZK IR module, verifies references against IR
metadata, and optionally emits the parsed AST for debugging or tests.

## Current Architecture

- `src/grammar/llzk_spec.pest` defines the concrete syntax.
- `src/ast.rs` defines the public AST shape and JSON serialization structure.
- `src/parser.rs` lowers pest parse trees into the AST and emits parser diagnostics.
- `src/ir.rs` parses LLZK IR with `llzk-rs`/MLIR and extracts metadata used by verification.
- `src/verify.rs` performs Phase 1 semantic checks: contract targets, symbol visibility,
  predicate/local scope rules, loop invariant binding arity, and contextual statement checks.
- `src/diagnostic.rs` owns user-facing diagnostics and top-level compile errors.
- `src/cli.rs` and `src/main.rs` implement the command-line interface.

The compiler deliberately normalizes parser, verifier, and IR-loading failures into
project diagnostics instead of exposing raw MLIR diagnostics as the public API.

## Language Surface

The current language supports:

- `contract for <symbol> { ... }`
- top-level and nested `predicate` declarations, either block-bodied or expression-bodied
- `require`, `ensure`, `let`, `unused`, and `return`
- `compute`, `witness`, and `constrain` scoped statements or blocks
- `forall` and `exists` quantifiers over expression domains or ranges
- `len(expr)`, `arg[N]`, escaped identifiers such as `` `return` ``, array indexing, calls,
  conditionals, unary operators, and arithmetic/logical binary operators
- loop invariants with full loop binding lists:

```spec
invariant for loop0(lb, i, ub, stride) {
  decreases ub - i;
  step i == old(i) + stride;
  ensure i <= ub;
}
```

For `scf.for`, invariant bindings are lower bound, induction variable, upper bound,
step, then iter args. For `scf.while`, bindings are the loop-carried block arguments
in order. A string `loop_label` attribute is used as the spec-visible loop name when
present; otherwise unlabeled loops get generated zero-indexed names like `loop0`,
`loop1`, etc. in IR walk order within each `struct.def`, or within a free
`function.def` when the loop is not inside a struct. Loops inside `poly.expr` do
not receive generated spec names.

`increases`, `decreases`, and `step` are only valid inside invariant bodies.
`old(expr)` is only valid inside a `step` expression. `step` and `old` are contextual
syntax rather than globally reserved identifiers.

`arg[N]` is a temporary escape hatch for unnamed LLZK function inputs. Keep the TODO
in `src/verify.rs` in mind: future LLZK metadata may allow source argument names to
replace most `arg[N]` usage.

## Tests

End-to-end tests are cargo-integrated lit-style tests:

- Specs live under `tests/lit/**/*.spec`.
- LLZK fixtures live under `tests/lit/Inputs`.
- `build.rs` discovers lit specs and `tests/lit.rs` generates one Rust test per file.
- Tests use `// RUN:` and `// CHECK:` comments, but developers run them through Cargo,
  not Python `lit`.
- Use `FileCheck` in lit specs for AST and diagnostic assertions.

Doc examples from the old language proposal have been extracted into
`tests/lit/doc-example-*.spec` with matching LLZK fixtures. Add new language behavior as
focused lit tests when possible, and add parser/verifier unit tests for narrow lowering
or semantic edge cases.

## Documentation Upkeep

- `docs/SYNTAX.md` is the hand-maintained user guide for writing specs.
- When a language feature is added, changed, or removed, update `docs/SYNTAX.md`
  in the same change.
- Every newly documented author-facing feature should have lit coverage.
- If a lit change alters author-facing behavior, review the guide and update it
  if needed.
- Prefer guide examples that match or are closely derived from `tests/lit/*.spec`.
- Do not try to autogenerate the guide from the grammar alone; the important
  behavior here includes naming rules, scope rules, and Phase 1 caveats that are
  not captured well by syntax extraction.

## Development Workflow

Use the Nix shell for normal development because the crate depends on the LLZK/MLIR
toolchain:

```sh
nix develop
cargo test
```

Plain `cargo test` outside Nix is expected to fail if `llvm-config` for LLVM 20 is not
available. The dev shell also provides a `FileCheck` symlink for lit tests.

Useful checks:

```sh
nix develop -c cargo test --test lit
nix develop -c cargo test
nix develop -c pre-commit run --all-files --hook-stage manual
```

Pre-commit hooks may modify files locally during normal commits, while the manual/CI
stage is read-only.

## Implementation Notes

- Prefer extending the existing grammar/AST/parser/verifier flow instead of adding
  ad hoc parsing or CLI-only behavior.
- Keep diagnostics user-facing and stable enough for lit tests to assert meaningful
  substrings.
- Preserve escaped identifier lowering: backticks are source syntax only, and the AST
  stores the unescaped symbol name.
- Treat `reserved_keyword` carefully. Reserved words should be rejected only as whole
  bare identifiers, not as prefixes like `for_label`.
- Do not reintroduce direct Python `lit` as a test dependency.
- Future work will lower the AST into the planned `verif` MLIR dialect; Phase 1 is
  parser, metadata extraction, structural verification, and AST emission only.
