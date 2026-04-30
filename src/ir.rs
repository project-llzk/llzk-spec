//! Methods for interacting with the LLZK IR.
//! Currently, until the `verif` dialect is implemented, these methods are limited
//! to LLZK IR loading and metadata extraction.
//! In future iterations, we will add functionality to emit `verif` dialect IR,
//! either combined with the LLZK IR or in a separate file

use crate::diagnostic::CompileError;
use llzk::context::LlzkContext;
use llzk::dialect::{
    function::is_func_def,
    poly::{is_expr_op, is_param_op, is_template_op},
    r#struct::{is_struct_def, is_struct_member},
};
use llzk::operation::WalkOperationMutLike;
use melior::{
    dialect::DialectRegistry,
    ir::{
        Module, OperationRef,
        attribute::StringAttribute,
        operation::{OperationLike, WalkOrder, WalkResult},
    },
    utility::register_all_dialects,
};
use mlir_sys::mlirOperationGetParentOperation;
use std::collections::{HashMap, HashSet};

/// Kind of loop operation discovered in LLZK IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopKind {
    For,
    While,
}

/// Function context for loop names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoopScope {
    Struct(String),
    Function(String),
}

/// Metadata needed to verify a spec invariant against an LLZK loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopMetadata {
    pub kind: LoopKind,
    pub binding_count: usize,
    pub scope: LoopScope,
    pub explicit_label: bool,
}

/// Symbol metadata collected from a parsed LLZK IR module.
#[derive(Debug, Clone)]
pub struct IrMetadata {
    /// Top-level names explicitly defined in the IR.
    pub defined_symbols: HashSet<String>,
    /// Any symbol names visible by reference in the IR.
    pub visible_names: HashSet<String>,
    /// Explicit `loop_label` loops and generated `loopN` loops,
    /// scoped to their containing struct or free function.
    pub labeled_loops: HashMap<(LoopScope, String), LoopMetadata>,
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

    extract_metadata(source_name, &mut module)
}

/// Walks the parsed IR and collects the names needed by semantic verification.
fn extract_metadata(
    source_name: &str,
    module: &mut Module<'_>,
) -> Result<IrMetadata, CompileError> {
    let mut metadata = IrMetadata {
        defined_symbols: HashSet::new(),
        visible_names: HashSet::new(),
        labeled_loops: HashMap::new(),
    };
    let mut duplicate_loop_name = None;
    let mut loop_indices = HashMap::<LoopScope, usize>::new();

    // TODO: does this have to be as mut?
    module
        .as_operation_mut()
        .walk_mut(WalkOrder::PreOrder, |operation| {
            if let Some(symbol_name) = string_attribute(&operation, "sym_name") {
                metadata.visible_names.insert(symbol_name.clone());
                if defines_symbol(&operation) {
                    metadata.defined_symbols.insert(symbol_name);
                }
            }

            if let Some((scope, kind, binding_count)) = loop_metadata(&operation) {
                if let Some(loop_name) = string_attribute(&operation, "loop_label") {
                    if metadata
                        .labeled_loops
                        .insert(
                            (scope.clone(), loop_name.clone()),
                            LoopMetadata {
                                kind,
                                binding_count,
                                scope,
                                explicit_label: true,
                            },
                        )
                        .is_some()
                    {
                        duplicate_loop_name = Some(loop_name);
                        return WalkResult::Interrupt;
                    }
                } else {
                    let index = loop_indices.entry(scope.clone()).or_default();
                    let loop_name = format!("loop{index}");
                    *index += 1;
                    metadata.labeled_loops.insert(
                        (scope.clone(), loop_name),
                        LoopMetadata {
                            kind,
                            binding_count,
                            scope,
                            explicit_label: false,
                        },
                    );
                }
            }

            WalkResult::Advance
        });

    if let Some(loop_name) = duplicate_loop_name {
        Err(CompileError::Ir(format!(
            "{source_name}: duplicate loop name `{loop_name}`"
        )))
    } else {
        Ok(metadata)
    }
}

/// Finds the owner scope for a loop. Struct methods use the nearest `struct.def`,
/// standalone functions use the nearest `function.def`, and loops inside `poly.expr`
/// are intentionally ignored.
fn containing_loop_scope<'c: 'a, 'a>(operation: &impl OperationLike<'c, 'a>) -> Option<LoopScope> {
    // TODO: Once we update melior, we shouldn't need these unsafe handlers.
    fn get_parent<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> Option<OperationRef<'c, 'a>> {
        let raw_parent = unsafe { mlirOperationGetParentOperation(op.to_raw()) };
        (!raw_parent.ptr.is_null()).then_some(unsafe { OperationRef::from_raw(raw_parent) })
    }

    let mut opt_parent = get_parent(operation);
    let mut function_scope = None;
    while let Some(parent) = opt_parent {
        if is_expr_op(&parent) {
            return None;
        }
        if is_struct_def(&parent)
            && let Some(struct_name) = string_attribute(&parent, "sym_name")
        {
            return Some(LoopScope::Struct(struct_name));
        }
        if is_func_def(&parent)
            && let Some(function_name) = string_attribute(&parent, "sym_name")
        {
            function_scope = Some(LoopScope::Function(function_name));
        }
        opt_parent = get_parent(&parent);
    }
    function_scope
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
fn defines_symbol<'c: 'a, 'a>(operation: &impl OperationLike<'c, 'a>) -> bool {
    is_struct_def(operation)
        || is_struct_member(operation)
        || is_func_def(operation)
        || is_param_op(operation)
        || is_expr_op(operation)
        || is_template_op(operation)
}

/// Returns loop scope, kind, and expected invariant binding count for supported loop ops.
fn loop_metadata<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
) -> Option<(LoopScope, LoopKind, usize)> {
    let scope = containing_loop_scope(operation)?;
    let identifier = operation.name();
    let name = identifier.as_string_ref().as_str().ok()?;
    match name {
        // scf.for operands are lower bound, upper bound, step, and iter args.
        // The invariant also binds the induction variable, so add one.
        "scf.for" => Some((scope, LoopKind::For, operation.operand_count() + 1)),
        // scf.while operands are the loop-carried values that become block args.
        "scf.while" => Some((scope, LoopKind::While, operation.operand_count())),
        _ => None,
    }
}
