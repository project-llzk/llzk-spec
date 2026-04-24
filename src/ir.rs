//! LLZK IR loading and shallow metadata extraction.

use crate::diagnostic::CompileError;
use llzk::context::LlzkContext;
use melior::{dialect::DialectRegistry, ir::Module, utility::register_all_dialects};
use regex::Regex;
use std::collections::HashSet;

/// Symbol metadata collected from a parsed LLZK IR module.
#[derive(Debug, Clone)]
pub struct IrMetadata {
    /// Top-level names explicitly defined in the IR.
    pub defined_symbols: HashSet<String>,
    /// Any symbol names visible by reference in the IR text.
    pub visible_names: HashSet<String>,
    /// Loop labels extracted from loop attributes.
    pub loop_labels: HashSet<String>,
}

/// Parses an LLZK IR module and extracts the metadata needed by phase 1 verification.
pub fn load_ir(source_name: &str, source: &str) -> Result<IrMetadata, CompileError> {
    let context = LlzkContext::new_no_log();
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    context.set_allow_unregistered_dialects(true);

    let _module = Module::parse(&context, source)
        .ok_or_else(|| CompileError::Ir(format!("{source_name}: failed to parse LLZK IR")))?;

    Ok(IrMetadata {
        defined_symbols: extract_defined_symbols(source),
        visible_names: extract_all_symbols(source),
        loop_labels: extract_loop_labels(source),
    })
}

/// Extracts symbol definitions from IR text using a conservative textual scan.
fn extract_defined_symbols(source: &str) -> HashSet<String> {
    let regex = Regex::new(
        r"(?:poly\.template|poly\.param|struct\.def|struct\.member|function\.def|channel\.def)\s+@([A-Za-z_][A-Za-z0-9_]*)",
    )
    .expect("definition regex");
    regex
        .captures_iter(source)
        .filter_map(|captures| captures.get(1).map(|capture| capture.as_str().to_string()))
        .collect()
}

/// Extracts all symbol references from IR text.
fn extract_all_symbols(source: &str) -> HashSet<String> {
    let regex = Regex::new(r"@([A-Za-z_][A-Za-z0-9_]*)").expect("symbol regex");
    regex
        .captures_iter(source)
        .filter_map(|captures| captures.get(1).map(|capture| capture.as_str().to_string()))
        .collect()
}

/// Extracts loop labels from loop attributes in the IR text.
fn extract_loop_labels(source: &str) -> HashSet<String> {
    let regex = Regex::new(r#"loop_label\s*=\s*\"([^\"]+)\""#).expect("loop label regex");
    regex
        .captures_iter(source)
        .filter_map(|captures| captures.get(1).map(|capture| capture.as_str().to_string()))
        .collect()
}
