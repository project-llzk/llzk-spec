use clap::Parser;
use llzk_spec::cli::{Args, run};
use std::process::ExitCode;

/// Simple main entry to invoke the CLI runnner.
fn main() -> ExitCode {
    let args = Args::parse();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
