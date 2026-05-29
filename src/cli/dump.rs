//! Functions for dumping different representations of a spec into different formats.

use crate::{ast::Document, cli::EmitFormat, diagnostic::CompileError};
use llzk::prelude::ModuleExt;
use melior::ir::Module;
use std::{
    fs::File,
    io::{Write, stdout},
    path::PathBuf,
};

/// Writes the AST to a file or stdout in the requested format.
pub fn write_ast(
    document: &Document,
    path: Option<&PathBuf>,
    format: EmitFormat,
) -> Result<(), CompileError> {
    let mut file = path_to_write(path)?;
    match format {
        EmitFormat::Debug => Ok(writeln!(&mut file, "{document:#?}")?),
        EmitFormat::Json => serde_json::to_writer_pretty(&mut file, document)
            .map_err(CompileError::AstSerialization),
        EmitFormat::Bytecode => Err(CompileError::Cli(
            "--emit-format=bytecode is not valid for --emit=ast".to_owned(),
        )),
    }
}

/// Writes the IR to a file or stdout in the requested format.
pub fn write_ir(
    module: &Module,
    path: Option<&PathBuf>,
    format: EmitFormat,
) -> Result<(), CompileError> {
    let mut file = path_to_write(path)?;
    match format {
        EmitFormat::Debug => {
            let op = module.as_operation();
            Ok(writeln!(&mut file, "{op}").map_err(CompileError::Io)?)
        }
        EmitFormat::Json => Err(CompileError::Cli(
            "--emit-format=json is not valid for --emit=ir".to_owned(),
        )),
        EmitFormat::Bytecode => Ok(module.write_bytecode(&mut file)?),
    }
}

/// Abstracts the actual write destination via the `Write` trait.
fn path_to_write(path: Option<&PathBuf>) -> Result<Box<dyn Write>, CompileError> {
    Ok(match path {
        None => Box::new(stdout()),
        Some(path) if path.as_os_str() == "-" => Box::new(stdout()),
        Some(path) => Box::new(File::create(path)?),
    })
}
