//! Functions for dumping different representations of a spec into different formats.

use crate::ast::Document;
use crate::cli::EmitFormat;
use crate::diagnostic::CompileError;
use melior::ir::Module;
use std::fs::File;
use std::io::{self, Write, stdout};
use std::os::raw::c_void;
use std::path::PathBuf;

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
        EmitFormat::Bytecode => write_bytecode(module, &mut file),
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

/// Write the generated `Module` to a file in bytecode format.
fn write_bytecode(module: &Module, dest: &mut dyn Write) -> Result<(), CompileError> {
    struct Wrap<'w>(&'w mut dyn Write, io::Result<()>);

    unsafe extern "C" fn callback(string_ref: mlir_sys::MlirStringRef, user_data: *mut c_void) {
        let wrap = unsafe { &mut *(user_data as *mut Wrap) };
        if wrap.1.is_err() {
            return;
        }
        let slice =
            unsafe { std::slice::from_raw_parts(string_ref.data as *const u8, string_ref.length) };
        wrap.1 = wrap.0.write_all(slice);
    }

    let mut wrap = Wrap(dest, Ok(()));
    unsafe {
        mlir_sys::mlirOperationWriteBytecode(
            module.as_operation().to_raw(),
            Some(callback),
            &mut wrap as *mut Wrap as *mut c_void,
        );
    }
    Ok(wrap.1?)
}
