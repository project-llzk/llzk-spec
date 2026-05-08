//! Cargo-integrated lit-style test harness.
//!
//! Each `.spec` file under `tests/lit` contains `// RUN:` directives that this
//! harness executes as normal Rust integration tests. The syntax intentionally
//! mirrors a small subset of LLVM lit syntax.
//!
//! Adapted from the lit harness in the Circom-to-llzk frontend: https://github.com/project-llzk/circom/blob/llzk/circom/tests/lit.rs

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{Builder, NamedTempFile, TempDir};

const TEST_INPUT: &str = "%s";
const TEST_SOURCE_DIR: &str = "%S";
const TMP_DIR: &str = "%t";
const LLZK_SPEC: &str = "%llzk_spec";

type LitResult<T> = Result<T, Box<dyn Error>>;

/// Extracts `// RUN:` directives until the optional `// END.` marker.
fn extract_runs(content: &str, source_path: &str) -> LitResult<Vec<String>> {
    let mut runs = Vec::new();
    for line in content.lines() {
        if is_end_directive(line) {
            break;
        }
        if let Some(run) = directive_value(line, "RUN") {
            runs.push(run.to_owned());
        }
    }

    if runs.is_empty() {
        Err(format!("lit test `{source_path}` is missing a RUN directive").into())
    } else {
        Ok(runs)
    }
}

/// Returns the value following a line comment directive such as `// RUN:`.
fn directive_value<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix("//")?.trim_start();
    let rest = rest.strip_prefix(directive)?;
    let rest = rest.strip_prefix(':')?;
    Some(rest.trim_start())
}

/// Returns whether a line is the directive terminator.
fn is_end_directive(line: &str) -> bool {
    line.trim_start()
        .strip_prefix("//")
        .map(|rest| rest.trim() == "END.")
        .unwrap_or(false)
}

/// Writes the checked-in spec text to a temporary file used for `%s`.
fn write_test(content: &str, source_path: &str) -> LitResult<NamedTempFile> {
    let file = Builder::new()
        .prefix(&temp_prefix(source_path))
        .suffix(".spec")
        .tempfile()?;
    fs::write(file.path(), content)?;
    Ok(file)
}

/// Builds a filesystem-friendly temp file prefix from a test path.
fn temp_prefix(source_path: &str) -> String {
    source_path
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Returns the checked-in directory containing a lit-style test file.
fn source_dir(source_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(source_path)
        .parent()
        .expect("lit test has a parent directory")
        .to_path_buf()
}

/// Quotes a filesystem path for safe insertion into a shell command.
fn shell_quote(path: &Path) -> String {
    let path = path.to_str().expect("test paths are valid UTF-8");
    format!("'{}'", path.replace('\'', "'\\''"))
}

/// Fully prepared lit-style test case.
struct LitTest {
    source_path: String,
    source_dir: PathBuf,
    run_commands: Vec<String>,
    test_input: NamedTempFile,
    tmp_dir: TempDir,
}

impl LitTest {
    /// Creates the temporary files and command list for one checked-in test.
    fn create(content: &str, source_path: &str) -> LitResult<Self> {
        Ok(Self {
            source_path: source_path.to_owned(),
            source_dir: source_dir(source_path),
            run_commands: extract_runs(content, source_path)?,
            test_input: write_test(content, source_path)?,
            tmp_dir: TempDir::new()?,
        })
    }

    /// Applies lit-style substitutions to one `RUN` command.
    fn prepare_command(&self, run_command: &str) -> String {
        run_command
            .replace(
                LLZK_SPEC,
                &shell_quote(Path::new(env!("CARGO_BIN_EXE_llzk-spec"))),
            )
            .replace(TEST_SOURCE_DIR, &shell_quote(&self.source_dir))
            .replace(TEST_INPUT, &shell_quote(self.test_input.path()))
            .replace(TMP_DIR, &shell_quote(self.tmp_dir.path()))
    }

    /// Executes every `RUN` command in order.
    fn execute(&self) -> LitResult<()> {
        for run_command in &self.run_commands {
            let command = self.prepare_command(run_command);
            let script = format!(
                r#"set -o pipefail
not() {{
  "$@"
  status=$?
  if [ "$status" -eq 0 ]; then
    echo "not: command unexpectedly succeeded" >&2
    return 1
  fi
  return 0
}}
{command}
"#
            );

            let output = Command::new("bash")
                .arg("-o")
                .arg("pipefail")
                .arg("-c")
                .arg(&script)
                .output()?;

            if !output.status.success() {
                return Err(format!(
                    "lit command failed in `{}`:\n{}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                    self.source_path,
                    command,
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }

        Ok(())
    }
}

/// Runs a checked-in lit-style test file.
fn lit_test(content: &str, source_path: &str) -> LitResult<()> {
    LitTest::create(content, source_path)?.execute()
}

include!(concat!(env!("OUT_DIR"), "/discovered_lit_tests.rs"));
