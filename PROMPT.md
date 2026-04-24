# Goal

The goal is to build a compiler for the llzk-spec language, in Rust, using the pest
parser and llzk-rs LLZK bindings.

This is the first phase of development. In this first phase, we will:
- Set up a nix flake so we can build llzk-rs.
- Create a rust tool for compiling llzk

# The Compiler

The llzk-spec compiler takes two files as input:
- The llzk-spec file
- The llzk IR file

The compiler will then parse the spec into an AST.
The compiler will load the LLZK IR file.
The compiler will check that the symbols defined in the spec correspond to symbols
in the llzk IR file.

In a later development stage, we will compile the llzk-spec AST into another MLIR
dialect. Since this `verif` dialect has not been created yet, development in this
phase will be limited to:
- Create the llzk-spec AST
- Check correspondence with the LLZK IR
- allow for the AST to be written to a file

# Testing

The compiler should be well tested. Unit tests should be present in the rust code where applicable.
We should also have lit-style tests for the compiler to test end-to-end functionality

# Development

Development should occur within the nix dev shell (using `nix develop`). This
is needed for llzk-rs to function properly, due to its LLVM/MLIR dependences.

# Completion

For this task to be complete, we need the following:
- A functional nix flake
- Unit and end-to-end tests
- A pre-commit setup that includes pre-commit checks for missing newlines at the end of files and rust formatting
- A CI GitHub workflow that runs the pre-commit checks and all tests
- The compiler with AST lowering and verification checks
