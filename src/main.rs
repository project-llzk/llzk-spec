use clap::Parser;
use llzk_spec::cli::{Args, run};
use llzk_spec::diagnostic::CompileError;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            print_error(&error);
            ExitCode::FAILURE
        }
    }
}

fn print_error(error: &CompileError) {
    let diagnostics = error.diagnostics();
    if diagnostics.is_empty() {
        eprintln!("{error}");
    } else {
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
        }
    }
}
