//! Methods for interacting with the LLZK IR.
//! Currently, until the `verif` dialect is implemented, these methods are limited
//! to LLZK IR loading and metadata extraction.
//! In future iterations, we will add functionality to emit `verif` dialect IR,
//! either combined with the LLZK IR or in a separate file

use crate::diagnostic::CompileError;
use llzk::context::LlzkContext;
use llzk::operation::WalkOperationMutLike;
use melior::{
    dialect::DialectRegistry,
    ir::{
        Module,
        attribute::StringAttribute,
        operation::{OperationLike, WalkOrder, WalkResult},
    },
    utility::register_all_dialects,
};
use std::collections::HashSet;

/// Symbol metadata collected from a parsed LLZK IR module.
#[derive(Debug, Clone)]
pub struct IrMetadata {
    /// Top-level names explicitly defined in the IR.
    pub defined_symbols: HashSet<String>,
    /// Any symbol names visible by reference in the IR.
    pub visible_names: HashSet<String>,
    /// Loop labels extracted from loop attributes.
    pub loop_labels: HashSet<String>,
}

/// Parses an LLZK IR module and extracts the metadata needed for symbol verification.
pub fn load_ir(source_name: &str, source: &str) -> Result<IrMetadata, CompileError> {
    let context = LlzkContext::new_no_log();
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    let mut module = Module::parse(&context, source)
        .ok_or_else(|| CompileError::Ir(format!("{source_name}: failed to parse LLZK IR")))?;

    Ok(extract_metadata(&mut module))
}

/// Walks the parsed IR and collects the names needed by semantic verification.
fn extract_metadata(module: &mut Module<'_>) -> IrMetadata {
    let mut metadata = IrMetadata {
        defined_symbols: HashSet::new(),
        visible_names: HashSet::new(),
        loop_labels: HashSet::new(),
    };

    module
        .as_operation_mut()
        .walk_mut(WalkOrder::PreOrder, |operation| {
            let operation_name = operation.name();
            let operation_name = operation_name
                .as_string_ref()
                .as_str()
                .expect("valid operation name");

            if let Some(symbol_name) = string_attribute(&operation, "sym_name") {
                metadata.visible_names.insert(symbol_name.clone());
                if defines_symbol(operation_name) {
                    metadata.defined_symbols.insert(symbol_name);
                }
            }

            if let Some(loop_label) = string_attribute(&operation, "loop_label") {
                metadata.loop_labels.insert(loop_label);
            }

            WalkResult::Advance
        });

    metadata
}

/// Returns the string value for a string-valued operation attribute when present.
fn string_attribute<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
    name: &str,
) -> Option<String> {
    operation
        .attribute(name)
        .ok()
        .and_then(|attribute| StringAttribute::try_from(attribute).ok())
        .map(|attribute| attribute.value().to_string())
}

/// Returns whether an operation contributes a new named symbol to the module.
fn defines_symbol(operation_name: &str) -> bool {
    matches!(
        operation_name,
        "poly.template"
            | "poly.param"
            | "struct.def"
            | "struct.member"
            | "function.def"
            | "channel.def"
    )
}
