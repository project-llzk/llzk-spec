//! Command-line interface for the `llzk-spec` compiler.

use crate::ast::AstContext;
use crate::diagnostic::CompileError;
use crate::ir::Context;
use crate::ir::llzk::load_ir;
use crate::ir::verif::emit_on_empty_module;
use crate::parser::parse_document;
use crate::verify::verify_document;
use clap::{Parser, ValueEnum};
use llzk::prelude::OperationLike;
use std::fs;
use std::path::PathBuf;

mod dump;

/// Emit actions supported by the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Emit {
    /// Emit the AST representation of the spec.
    Ast,
    /// Emit the MLIR IR representation of the spec.
    Ir,
}

/// Command-line arguments accepted by the compiler binary.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    /// Path to the `llzk-spec` source file.
    #[arg(long = "spec", alias = "spec-file", value_name = "SPEC_FILE")]
    pub spec_file: PathBuf,
    /// Path to the LLZK IR file used for symbol verification.
    #[arg(long = "llzk", alias = "llzk-ir-file", value_name = "LLZK_IR_FILE")]
    pub llzk_ir_file: PathBuf,
    /// Optional AST output path, or `-` for stdout.
    #[arg(long)]
    pub emit: Option<Emit>,
    /// Output format used when `--emit-ast` is set.
    #[arg(long, default_value = "debug")]
    pub emit_format: EmitFormat,
    /// Output destination for `--emit`. Defaults to stdout if omitted or if given '-'.
    #[arg(long)]
    pub emit_dest: Option<PathBuf>,
    /// Prime field used by the circuit.
    #[arg(long)]
    pub field: Option<String>,
}

/// Supported AST serialization formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EmitFormat {
    /// Dumps with debug formatting (for `--emit=ir` dumps the IR in plaintext).
    Debug,
    /// Dumps in JSON format (only valid for `--emit=ast`).
    Json,
    /// Dumps the IR in binary format (only valid for `--emit=ir`).
    Bytecode,
}

/// Runs the CLI pipeline: parse, load IR, verify, and optionally emit the AST.
pub fn run(args: Args) -> Result<(), CompileError> {
    let spec_source = fs::read_to_string(&args.spec_file)?;
    let ir_source = fs::read_to_string(&args.llzk_ir_file)?;

    let spec_name = args.spec_file.display().to_string();
    let ir_name = args.llzk_ir_file.display().to_string();

    let ctx = AstContext::new();
    let document = parse_document(&ctx, &spec_name, &spec_source).map_err(CompileError::Syntax)?;

    if let Some(Emit::Ast) = args.emit {
        dump::write_ast(&document, args.emit_dest.as_ref(), args.emit_format)?;
    }

    // Wrap emitting the MLIR IR of the spec in `--emit=ir` since the verification logic currently
    // uses the AST.
    if let Some(Emit::Ir) = args.emit {
        let ir_ctx = args.field.map(Context::with_field).unwrap_or_default();
        let circuit = ir_ctx.parse_module(&ir_name, &ir_source)?;
        let module = emit_on_empty_module(&ir_ctx, &ctx, &spec_name, &document, &circuit)?;
        if !module.as_operation().verify() {
            return Err(CompileError::Ir(format!(
                "spec module failed to verify\nModule:\n{}",
                module.as_operation()
            )));
        }
        dump::write_ir(&module, args.emit_dest.as_ref(), args.emit_format)?;

        return Ok(());
    }

    let ir = load_ir(&ir_name, &ir_source)?;
    verify_document(&document, &ir, &spec_name)
        .map_err(|diagnostics| CompileError::Diagnostics(diagnostics.into()))?;

    Ok(())
}
