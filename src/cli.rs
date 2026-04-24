use crate::ast::Document;
use crate::diagnostic::CompileError;
use crate::ir::load_ir;
use crate::parser::parse_document;
use crate::verify::verify_document;
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    pub spec_file: PathBuf,
    pub llzk_ir_file: PathBuf,
    #[arg(long)]
    pub emit_ast: Option<PathBuf>,
    #[arg(long, default_value = "debug")]
    pub format: EmitFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EmitFormat {
    Debug,
    Json,
}

pub fn run(args: Args) -> Result<(), CompileError> {
    let spec_source = fs::read_to_string(&args.spec_file)?;
    let ir_source = fs::read_to_string(&args.llzk_ir_file)?;

    let spec_name = args.spec_file.display().to_string();
    let ir_name = args.llzk_ir_file.display().to_string();

    let document = parse_document(&spec_name, &spec_source).map_err(CompileError::Syntax)?;
    let ir = load_ir(&ir_name, &ir_source)?;
    verify_document(&document, &ir, &spec_name).map_err(CompileError::Diagnostics)?;

    if let Some(path) = args.emit_ast {
        write_ast(&document, &path, args.format)?;
    }

    Ok(())
}

fn write_ast(document: &Document, path: &PathBuf, format: EmitFormat) -> Result<(), CompileError> {
    let payload = match format {
        EmitFormat::Debug => format!("{document:#?}\n"),
        EmitFormat::Json => {
            let mut json = serde_json::to_string_pretty(document).map_err(|error| {
                CompileError::Usage(format!("failed to serialize AST: {error}"))
            })?;
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
