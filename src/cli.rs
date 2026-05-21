//! Command-line interface for the `llzk-spec` compiler.

use crate::ast::{AstContext, Document};
use crate::diagnostic::CompileError;
use crate::ir::load_ir;
use crate::parser::parse_document;
use crate::verify::verify_document;
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

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
    pub emit_ast: Option<PathBuf>,
    /// Output format used when `--emit-ast` is set.
    #[arg(long, default_value = "debug")]
    pub format: EmitFormat,
}

/// Supported AST serialization formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EmitFormat {
    Debug,
    Json,
}

/// Runs the CLI pipeline: parse, load IR, verify, and optionally emit the AST.
pub fn run(args: Args) -> Result<(), CompileError> {
    let spec_source = fs::read_to_string(&args.spec_file)?;
    let ir_source = fs::read_to_string(&args.llzk_ir_file)?;

    let spec_name = args.spec_file.display().to_string();
    let ir_name = args.llzk_ir_file.display().to_string();

    let ast_ctx = AstContext::new();
    let document =
        parse_document(&ast_ctx, &spec_name, &spec_source).map_err(CompileError::Syntax)?;
    let ir = load_ir(&ir_name, &ir_source)?;
    verify_document(&document, &ir, &spec_name)
        .map_err(|diagnostics| CompileError::Diagnostics(diagnostics.into()))?;

    if let Some(path) = args.emit_ast {
        write_ast(&document, &path, args.format)?;
    }

    Ok(())
}

/// Writes the AST to a file or stdout in the requested format.
fn write_ast(document: &Document, path: &PathBuf, format: EmitFormat) -> Result<(), CompileError> {
    let payload = match format {
        EmitFormat::Debug => format!("{document:#?}\n"),
        EmitFormat::Json => {
            let mut json =
                serde_json::to_string_pretty(document).map_err(CompileError::AstSerialization)?;
            json.push('\n');
            json
        }
    };

    if path.as_os_str() == "-" {
        print!("{payload}");
    } else {
        fs::write(path, payload)?;
    }

    Ok(())
}
