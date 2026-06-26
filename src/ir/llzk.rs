//! Methods for interacting with the LLZK IR.
//! Currently, until the `verif` dialect is implemented, these methods are limited
//! to LLZK IR loading and metadata extraction.
//! In future iterations, we will add functionality to emit `verif` dialect IR,
//! either combined with the LLZK IR or in a separate file.

use crate::ast::Identifier;
use crate::diagnostic::CompileError;
use crate::ir::MlirTypeSystem;
use crate::type_analysis::loops::{LoopInfo, LoopLabel};
use crate::type_analysis::{
    CircuitInfo, ContractTargetInfo, InputInfo, MemberInfo, OutputInfo, ParamInfo,
};
use llzk::dialect::{
    array::ArrayType,
    function::{FuncDefOpLike, is_func_def},
    poly::is_expr_op,
    r#struct::{MemberDefOpLike, StructDefOpLike, StructType, is_struct_def},
};
use llzk::operation::WalkOperationMutLike;
use llzk::prelude::{
    FuncDefOpRef, FuncDefOpRefMut, OperationMutLike, PodType, StructDefOpRef, SymbolRefAttribute,
    TemplateOpLike, TemplateOpRef, TemplateSymbolBindingOpLike as _,
};
use melior::ir::{
    Module, OperationRef, Type,
    attribute::{FlatSymbolRefAttribute, StringAttribute},
    operation::{OperationLike, WalkOrder, WalkResult},
};
use melior::ir::{TypeLike as _, ValueLike};
use mlir_sys::mlirOperationGetParentOperation;
use std::collections::{HashMap, HashSet};

mod error;

/// Implementation of [`CircuitInfo`] for LLZK circuits.
#[derive(Copy, Clone)]
pub struct LlzkInfo<'ctx, 'm> {
    /// The MLIR module that defines the LLZK circuit.
    module: &'m Module<'ctx>,
}

impl<'ctx, 'm> LlzkInfo<'ctx, 'm> {
    /// Creates a new info provider.
    pub fn new(module: &'m Module<'ctx>) -> Self {
        Self { module }
    }
}

/// Returns true if the given identifier is equivalent to the symbol.
///
/// The name must be a fully qualified name with its parts separated by `::`. Each part must be
/// equal to each part of the symbol.
fn check_fqn<M>(name: &Identifier<M>, fqn: SymbolRefAttribute) -> bool {
    let name_parts = name.value().split("::").collect::<Vec<_>>();
    let Ok(root) = fqn.root().as_str() else {
        // If the root is not a UTF8 string then we consider it not equal since we expect the identifier to
        // contain valid unicode.
        return false;
    };
    let fqn_parts = std::iter::once(root)
        .chain(fqn.nested().into_iter().map(|s| s.value()))
        .collect::<Vec<_>>();

    name_parts == fqn_parts
}

fn collect_function_input_names<'c, 'a>(
    func: FuncDefOpRef<'c, 'a>,
    source_offset: usize,
    names: &mut HashSet<String>,
) {
    let arg_count = func
        .function_type()
        .map(|t| t.input_count())
        .unwrap_or_default();
    for n in source_offset..arg_count {
        if let Some(name) = function_input_name(func, n) {
            names.insert(name.to_string());
        }
    }
}

/// Attempts to extract an op of the given type if the fqn matches
macro_rules! extract_target {
    ($op:ident, $name:expr, $of_ref:ty, $target:ident, $result:ident) => {
        if let Some($op) = <$of_ref>::from_option_raw($op.to_raw()) {
            if check_fqn($name, $op.fully_qualified_name()) {
                $result = Some(LlzkContractTarget::$target($op));
                return WalkResult::Interrupt;
            } else {
                // Don't walk inside the operation since there isn't anything interesting to
                // look at.
                return WalkResult::Skip;
            }
        }
    };
}

impl<'ctx, 'm> CircuitInfo<'ctx> for LlzkInfo<'ctx, 'm> {
    type Error = error::Error;

    type TypeSystem = MlirTypeSystem<'ctx, 'm>;

    fn find_contract_target<M>(
        &self,
        name: &Identifier<M>,
    ) -> Result<impl ContractTargetInfo<'ctx, TypeSystem = Self::TypeSystem>, Self::Error> {
        let mut result = None;
        self.module
            .as_operation()
            .walk(WalkOrder::PreOrder, |operation| {
                extract_target!(operation, name, StructDefOpRef, Struct, result);
                extract_target!(operation, name, FuncDefOpRef, Function, result);

                WalkResult::Advance
            });

        result.ok_or_else(|| error::Error::ContractTargetNotFound(name.value().to_owned()))
    }
}

/// Implementation of [`ContractTargetInfo`] for LLZK structs and functions.
#[derive(Copy, Clone, Debug)]
pub(super) enum LlzkContractTarget<'ctx, 'op> {
    Struct(StructDefOpRef<'ctx, 'op>),
    Function(FuncDefOpRef<'ctx, 'op>),
}

impl<'ctx, 'op> LlzkContractTarget<'ctx, 'op> {
    /// Returns the fully qualified name of the target.
    pub fn fully_qualified_name(&self) -> SymbolRefAttribute<'ctx> {
        match self {
            LlzkContractTarget::Struct(op_ref) => op_ref.fully_qualified_name(),
            LlzkContractTarget::Function(op_ref) => op_ref.fully_qualified_name(),
        }
    }

    /// Returns the loops in the target.
    pub fn loops(&self) -> Vec<LlzkLoopTarget<'ctx, 'op>> {
        let mut loops = vec![];
        self.walk(WalkOrder::PreOrder, |op| {
            let i = op.name();
            match i.as_string_ref().as_str().ok() {
                Some("scf.for") => loops.push(LlzkLoopTarget::r#for(op, loops.len())),
                Some("scf.while") => loops.push(LlzkLoopTarget::r#while(op, loops.len())),
                _ => {}
            };
            WalkResult::Advance
        });
        loops
    }
}

impl<'ctx, 'op> From<LlzkContractTarget<'ctx, 'op>> for OperationRef<'ctx, 'op> {
    fn from(value: LlzkContractTarget<'ctx, 'op>) -> Self {
        match value {
            LlzkContractTarget::Struct(op_ref) => op_ref.into(),
            LlzkContractTarget::Function(op_ref) => op_ref.into(),
        }
    }
}

impl<'ctx, 'op> OperationLike<'ctx, 'op> for LlzkContractTarget<'ctx, 'op> {
    fn to_raw(&self) -> mlir_sys::MlirOperation {
        OperationRef::from(*self).to_raw()
    }
}

impl std::fmt::Display for LlzkContractTarget<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&OperationRef::from(*self), f)
    }
}

impl<'ctx, 'op> ContractTargetInfo<'ctx> for LlzkContractTarget<'ctx, 'op> {
    type TypeSystem = MlirTypeSystem<'ctx, 'op>;

    fn inputs(&self) -> impl Iterator<Item = InputInfo<'ctx, Type<'ctx>>> {
        let (f, source_offset) = match self {
            Self::Struct(op) => preferred_struct_input_func(*op).unwrap(),
            Self::Function(op) => (*op, 0),
        };
        let arg_count = f
            .function_type()
            .map(|t| t.input_count().saturating_sub(source_offset))
            .unwrap_or_default();
        (0..arg_count).map(move |n| {
            let source_idx = n + source_offset;
            let name = function_input_name(f, source_idx);
            let t = unsafe { Type::from_raw(f.argument(source_idx).unwrap().r#type().to_raw()) };

            match name {
                Some(name) => InputInfo::named(name, t),
                None => InputInfo::unnamed(t),
            }
        })
    }

    fn outputs(&self) -> impl Iterator<Item = OutputInfo<'ctx, Type<'ctx>>> {
        match self {
            LlzkContractTarget::Struct(_) => None,
            LlzkContractTarget::Function(op_ref) => Some(op_ref),
        }
        .into_iter()
        .flat_map(|op_ref| {
            let names = function_output_names(*op_ref);
            op_ref.function_type().into_iter().flat_map(move |t| {
                let count = t.result_count();
                let names = names.clone();
                (0..count).map(move |n| (t.result(n).unwrap(), names.get(n).copied().flatten()))
            })
        })
        .map(|(t, name)| match name {
            Some(name) => OutputInfo::named(name, t),
            None => OutputInfo::unnamed(t),
        })
    }

    fn members(&self) -> impl Iterator<Item = MemberInfo<'ctx, Type<'ctx>>> {
        match self {
            Self::Struct(op) => Some(op),
            Self::Function(_) => None,
        }
        .into_iter()
        .flat_map(|op| op.member_defs())
        .map(move |m| {
            MemberInfo::new(
                m.member_name(),
                unsafe { Type::from_raw(m.member_type().to_raw()) },
                m.has_public_attr(),
            )
        })
    }

    fn template_params(&self) -> impl Iterator<Item = ParamInfo<'ctx, Type<'ctx>>> {
        match self {
            Self::Struct(op) => op.parent_operation(),
            Self::Function(op) => op.parent_operation(),
        }
        .into_iter()
        .filter_map(|op| TemplateOpRef::try_from(op).ok())
        .flat_map(|op| op.const_binding_ops())
        .map(|op| {
            ParamInfo::new(
                op.sym_name(),
                op.type_opt().map(|t| unsafe { Type::from_raw(t.to_raw()) }),
            )
        })
    }

    fn loops(&self, ts: &mut Self::TypeSystem) -> Vec<LoopInfo<Type<'ctx>>> {
        self.loops()
            .into_iter()
            .map(|op| match op.kind {
                LoopKind::For => LoopInfo::new_for_loop(
                    op.label(),
                    |_| ts.index_type(),
                    op.extra_operands_types(),
                ),
                LoopKind::While => LoopInfo::new_while_loop(op.label(), op.extra_operands_types()),
            })
            .collect()
    }

    fn self_type(&self) -> Option<Type<'ctx>> {
        match self {
            Self::Struct(op) => Some(fix_struct_type(op.r#type()).into()),
            Self::Function(_) => None,
        }
    }
}

/// Convert `!struct.type<@Foo>` into `!struct.type<@Foo<[]>>`.
fn fix_struct_type<'ctx>(s: StructType<'ctx>) -> StructType<'ctx> {
    if s.params().is_none() {
        StructType::new(s.name(), &[])
    } else {
        s
    }
}

/// Reference to a loop.
#[derive(Debug, Copy, Clone)]
pub(super) struct LlzkLoopTarget<'ctx, 'op> {
    op: OperationRef<'ctx, 'op>,
    kind: LoopKind,
    idx: usize,
}

impl<'ctx, 'blk> LlzkLoopTarget<'ctx, 'blk> {
    /// Create a `for` variant.
    fn r#for(op: OperationRef, idx: usize) -> Self {
        Self {
            op: unsafe { OperationRef::from_raw(op.to_raw()) },
            kind: LoopKind::For,
            idx,
        }
    }

    /// Create a `while` variant.
    fn r#while(op: OperationRef, idx: usize) -> Self {
        Self {
            op: unsafe { OperationRef::from_raw(op.to_raw()) },
            kind: LoopKind::While,
            idx,
        }
    }

    /// Returns the label associated with the loop.
    pub fn label(&self) -> LoopLabel {
        self.attribute("loop_label")
            .ok()
            .and_then(|attribute| StringAttribute::try_from(attribute).ok())
            .map(|attribute| attribute.value())
            .map(LoopLabel::named)
            .unwrap_or(LoopLabel::implicit(self.idx))
    }

    /// Returns the extra operands the loop has.
    pub fn extra_operands_types(&self) -> impl Iterator<Item = Type<'ctx>> {
        // `scf.for` loops have 3 base operands (lb, up, stride) and then its extra arguments.
        // The types for those 3 operands + the inductive variable are handled
        // by the constructor.
        self.operands()
            .skip(match self.kind {
                LoopKind::For => 3,
                LoopKind::While => 0,
            })
            .map(|v| v.r#type())
    }

    /// Adds the loop label to the targeted loop if it wasn't present during analysis and had to
    /// be inferred.
    ///
    /// The verif.invariant operation expects the label is present during verification.
    pub fn ensure_label_is_present(&mut self) {
        if self.has_attribute("loop_label") {
            return;
        }
        let label = LoopLabel::implicit(self.idx);
        let context = self.context();
        let label = StringAttribute::new(unsafe { context.to_ref() }, &label.to_string());
        self.set_attribute("loop_label", label.into());
    }
}

impl<'ctx, 'op> OperationMutLike<'ctx, 'op> for LlzkLoopTarget<'ctx, 'op> {}

impl<'ctx, 'op> OperationLike<'ctx, 'op> for LlzkLoopTarget<'ctx, 'op> {
    fn to_raw(&self) -> mlir_sys::MlirOperation {
        self.op.to_raw()
    }
}

impl std::fmt::Display for LlzkLoopTarget<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.op, f)
    }
}

/// Kind of loop operation discovered in LLZK IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopKind {
    For,
    While,
}

/// Function context for loop names.
///
/// Different functions and structs can reuse the same loop labels, but loop
/// labels cannot be repeated in struct compute/constrain functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoopScope {
    Struct(String),
    Function(String),
}

/// Metadata needed to verify a spec invariant against an LLZK loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopMetadata {
    pub kind: LoopKind,
    /// Number of loop variables to bind.
    pub binding_count: usize,
    pub scope: LoopScope,
    /// Whether this loop was explicitly labeled in the IR or if the name was
    /// autogenerated.
    pub explicit_label: bool,
}

/// Contract-specific metadata derived from a contract target op (e.g., free function or struct).
#[derive(Debug, Default, Clone)]
struct ContractMetadata {
    visible_symbols: HashSet<String>,
    member_paths: HashMap<String, bool>,
}

/// Symbol metadata collected from a parsed LLZK IR module.
#[derive(Debug, Clone)]
pub struct IrMetadata {
    /// Contract targets (e.g., free functions and structs) explicitly defined in the IR.
    pub global_symbols: HashSet<String>,
    /// Names visible inside a contract, keyed by contract target.
    pub visible_symbols: HashMap<String, HashSet<String>>,
    /// Nested member paths visible from a contract target, keyed by contract target.
    pub member_paths: HashMap<String, HashMap<String, bool>>,
    /// Explicit `loop_label` loops and generated `loopN` loops,
    /// scoped to their containing struct or free function.
    pub labeled_loops: HashMap<(LoopScope, String), LoopMetadata>,
}

impl IrMetadata {
    /// Check if the IR contains the given global symbol.
    ///
    /// A global symbol is one that can have contracts written for it (i.e.,
    /// LLZK structs and functions).
    pub fn has_global_symbol(&self, name: &str) -> bool {
        self.global_symbols.contains(name)
    }

    /// Check if the `name` symbol is visible from the perspective of a contract
    /// written for `target`.
    pub fn symbol_visible_in_contract(&self, target: &str, name: &str) -> bool {
        self.visible_symbols
            .get(target)
            .is_some_and(|symbols| symbols.contains(name))
    }

    /// Check if a member with path `path` is visible from `target`.
    pub fn member_visibility(&self, target: &str, path: &str) -> Option<bool> {
        self.member_paths
            .get(target)
            .and_then(|paths| paths.get(path))
            .copied()
    }
}

/// Parses an LLZK IR module and extracts the metadata needed for symbol verification.
pub fn load_ir(source_name: &str, source: &str) -> Result<IrMetadata, CompileError> {
    let context = super::Context::new();
    let mut module = context.parse_module(source_name, source)?;

    extract_metadata(source_name, &mut module)
}

/// Walks the parsed IR and collects the names needed by semantic verification.
fn extract_metadata(
    source_name: &str,
    module: &mut Module<'_>,
) -> Result<IrMetadata, CompileError> {
    let mut metadata = IrMetadata {
        global_symbols: HashSet::new(),
        visible_symbols: HashMap::new(),
        member_paths: HashMap::new(),
        labeled_loops: HashMap::new(),
    };
    let mut duplicate_loop_name = None;
    let mut loop_indices = HashMap::<LoopScope, usize>::new();

    module
        .as_operation_mut()
        .walk_mut(WalkOrder::PreOrder, |operation| {
            // Collect all symbols from relevant symbol defining ops.
            if let Some(struct_op) = StructDefOpRef::from_option_raw(operation.to_raw())
                && let Some(struct_name) = string_attribute(&operation, "sym_name")
            {
                let target = struct_contract_target_name(&operation, &struct_name);
                metadata.global_symbols.insert(target.clone());
                let contract_metadata = collect_struct_contract_metadata(&struct_op);
                metadata
                    .visible_symbols
                    .insert(target.clone(), contract_metadata.visible_symbols);
                metadata
                    .member_paths
                    .insert(target, contract_metadata.member_paths);
            }

            if is_func_def(&operation)
                && let Some(function_name) = string_attribute(&operation, "sym_name")
            {
                let target = function_contract_target_name(&operation, &function_name);
                metadata.global_symbols.insert(target.clone());
                // Function may already be visible if it is a struct method, hence the check
                if !metadata.visible_symbols.contains_key(&target) {
                    let contract_metadata = collect_function_contract_metadata(
                        &operation,
                        &metadata.visible_symbols,
                        &metadata.member_paths,
                    );
                    metadata
                        .visible_symbols
                        .insert(target.clone(), contract_metadata.visible_symbols);
                    metadata
                        .member_paths
                        .insert(target, contract_metadata.member_paths);
                }
            }

            // Collect all the loop scopes
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
                    if metadata
                        .labeled_loops
                        .insert(
                            (scope.clone(), loop_name.clone()),
                            LoopMetadata {
                                kind,
                                binding_count,
                                scope,
                                explicit_label: false,
                            },
                        )
                        .is_some()
                    {
                        duplicate_loop_name = Some(loop_name);
                        return WalkResult::Interrupt;
                    }
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

/// Get the parent of a given operation, if it exists.
fn get_parent<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> Option<OperationRef<'c, 'a>> {
    // TODO: Once we update melior, we shouldn't need these unsafe handlers.
    let raw_parent = unsafe { mlirOperationGetParentOperation(op.to_raw()) };
    (!raw_parent.ptr.is_null()).then_some(unsafe { OperationRef::from_raw(raw_parent) })
}

/// Finds the owner scope for a loop. Struct methods use the nearest `struct.def`,
/// standalone functions use the nearest `function.def`, and loops inside `poly.expr`
/// are intentionally ignored.
fn containing_loop_scope<'c: 'a, 'a>(operation: &impl OperationLike<'c, 'a>) -> Option<LoopScope> {
    let mut opt_parent = get_parent(operation);
    let mut function_scope = None;
    while let Some(parent) = opt_parent {
        if is_expr_op(&parent) {
            return None;
        }
        if is_struct_def(&parent)
            && let Some(struct_name) = string_attribute(&parent, "sym_name")
        {
            let struct_symbol = struct_contract_target_name(&parent, &struct_name);
            return Some(LoopScope::Struct(struct_symbol));
        }
        if is_func_def(&parent)
            && let Some(function_name) = string_attribute(&parent, "sym_name")
        {
            let function_symbol = function_contract_target_name(&parent, &function_name);
            function_scope = Some(LoopScope::Function(function_symbol));
        }
        opt_parent = get_parent(&parent);
    }
    function_scope
}

/// Returns the enclosing named modules/templates from outermost to innermost.
fn symbol_ancestor_names<'c: 'a, 'a>(operation: &impl OperationLike<'c, 'a>) -> Vec<String> {
    let mut names = Vec::new();
    let mut opt_parent = get_parent(operation);
    while let Some(parent) = opt_parent {
        let op_name = parent.name();
        let op_name = op_name.as_string_ref();
        let op_name = op_name.as_str().ok();
        if matches!(op_name, Some("builtin.module" | "poly.template"))
            && let Some(module_name) = string_attribute(&parent, "sym_name")
        {
            names.push(module_name);
        }
        opt_parent = get_parent(&parent);
    }
    names.reverse();
    names
}

/// Returns the enclosing named builtin modules from outermost to innermost.
fn collect_struct_contract_metadata<'c: 'a, 'a>(
    struct_op: &StructDefOpRef<'c, 'a>,
) -> ContractMetadata {
    let mut metadata = ContractMetadata::default();

    for member in struct_op.member_defs() {
        let member_name = member.member_name().to_string();
        metadata.visible_symbols.insert(member_name.clone());
        collect_member_paths(
            struct_op,
            &member_name,
            member.member_type(),
            true,
            &mut metadata.member_paths,
        );
    }

    for name in struct_op
        .template_param_op_names()
        .into_iter()
        .chain(struct_op.template_expr_op_names())
    {
        metadata.visible_symbols.insert(flat_symbol_name(name));
    }
    metadata
        .visible_symbols
        .extend(collect_operation_input_names(struct_op));

    metadata
}

fn collect_function_contract_metadata<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
    visible_symbols: &HashMap<String, HashSet<String>>,
    member_paths: &HashMap<String, HashMap<String, bool>>,
) -> ContractMetadata {
    if let Some(struct_target) = containing_struct_target(operation) {
        return ContractMetadata {
            visible_symbols: visible_symbols
                .get(&struct_target)
                .cloned()
                .unwrap_or_default(),
            member_paths: member_paths
                .get(&struct_target)
                .cloned()
                .unwrap_or_default(),
        };
    }

    let mut metadata = ContractMetadata::default();
    if let Some(template) = containing_template_op(operation) {
        for name in template
            .const_param_names()
            .into_iter()
            .chain(template.const_expr_names())
        {
            metadata.visible_symbols.insert(flat_symbol_name(name));
        }
    }
    metadata
        .visible_symbols
        .extend(collect_operation_input_names(operation));
    metadata
        .visible_symbols
        .extend(collect_function_output_names(operation));
    metadata
}

/// Returns the struct entrypoint whose arguments should be exposed to specs.
///
/// Struct contracts prefer `constrain` when present because it carries the
/// concrete runtime inputs plus the leading struct operand. Product-only structs
/// expose the `product` inputs directly. As a final fallback, `compute` inputs
/// are used.
pub(crate) fn preferred_struct_input_func<'c: 'a, 'a>(
    struct_op: StructDefOpRef<'c, 'a>,
) -> Option<(FuncDefOpRef<'c, 'a>, usize)> {
    struct_op
        .constrain_func()
        .map(|func| (func, 1))
        .or_else(|| struct_op.product_func().map(|func| (func, 0)))
        .or_else(|| struct_op.compute_func().map(|func| (func, 0)))
}

/// Collects every spec-visible input name exported by the given callable target.
///
/// Free functions contribute all named inputs. Struct targets contribute the
/// names from the same preferred entrypoint used for contract lowering, skipping
/// any leading struct operand.
fn collect_operation_input_names<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
) -> HashSet<String> {
    let mut names = HashSet::new();

    if let Some(func) = FuncDefOpRef::from_option_raw(operation.to_raw()) {
        collect_function_input_names(func, 0, &mut names);
        return names;
    }

    if let Some(struct_op) = StructDefOpRef::from_option_raw(operation.to_raw()) {
        if let Some((func, source_offset)) = preferred_struct_input_func(struct_op) {
            collect_function_input_names(func, source_offset, &mut names);
        }
    }

    names
}

fn collect_function_output_names<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
) -> HashSet<String> {
    FuncDefOpRef::from_option_raw(operation.to_raw())
        .map(function_output_names)
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect()
}

/// Returns the user-facing name for a function input, if the IR exposes one.
///
/// Unnamed arguments remain addressable through spec-side `$arg[N]` syntax, so
/// this only reports names backed by the canonical `function.arg_name`
/// attribute.
pub(crate) fn function_input_name<'c, 'a>(
    func: FuncDefOpRef<'c, 'a>,
    idx: usize,
) -> Option<&'c str> {
    func.arg_name_attr(idx).ok().flatten().map(|a| a.value())
}

pub(crate) fn function_output_name<'c, 'a>(
    func: FuncDefOpRef<'c, 'a>,
    idx: usize,
) -> Option<&'c str> {
    func.res_name_attr(idx).ok().flatten().map(|a| a.value())
}

fn function_output_names<'c, 'a>(func: FuncDefOpRef<'c, 'a>) -> Vec<Option<&'c str>> {
    let count = func
        .function_type()
        .map(|t| t.result_count())
        .unwrap_or_default();
    (0..count)
        .map(|idx| function_output_name(func, idx))
        .collect()
}

/// Collect member reference paths to populate `member_paths`, using `root` as
/// the root for symbol lookups, when needed.
fn collect_member_paths<'c: 'a, 'a>(
    root: &impl OperationLike<'c, 'a>,
    prefix: &str,
    member_type: Type<'c>,
    parent_accessible: bool,
    member_paths: &mut HashMap<String, bool>,
) {
    if let Ok(struct_type) = StructType::try_from(member_type) {
        collect_struct_member_paths(root, prefix, struct_type, parent_accessible, member_paths);
    } else if let Ok(pod_type) = PodType::try_from(member_type) {
        collect_pod_member_paths(root, prefix, pod_type, parent_accessible, member_paths);
    } else if let Ok(array_type) = ArrayType::try_from(member_type) {
        collect_array_member_paths(root, prefix, array_type, parent_accessible, member_paths);
    }
}

fn collect_struct_member_paths<'c: 'a, 'a>(
    root: &impl OperationLike<'c, 'a>,
    prefix: &str,
    struct_type: StructType<'c>,
    parent_accessible: bool,
    member_paths: &mut HashMap<String, bool>,
) {
    let Ok(lookup) = struct_type.lookup_definition(root) else {
        return;
    };
    let Some(operation) = lookup.operation() else {
        return;
    };
    let Ok(struct_def) = StructDefOpRef::try_from(operation) else {
        return;
    };

    for member in struct_def.member_defs() {
        let path = format!("{prefix}.{}", member.member_name());
        let accessible = parent_accessible && member.has_public_attr();
        member_paths.insert(path.clone(), accessible);
        collect_member_paths(
            &member,
            &path,
            member.member_type(),
            accessible,
            member_paths,
        );
    }
}

fn collect_pod_member_paths<'c: 'a, 'a>(
    root: &impl OperationLike<'c, 'a>,
    prefix: &str,
    pod_type: PodType<'c>,
    parent_accessible: bool,
    member_paths: &mut HashMap<String, bool>,
) {
    for record in pod_type.records() {
        let record_name = record
            .name()
            .as_string_ref()
            .as_str()
            .unwrap_or("")
            .trim_start_matches('@')
            .to_string();
        let path = format!("{prefix}.{record_name}");
        member_paths.insert(path.clone(), parent_accessible);
        collect_member_paths(
            root,
            &path,
            record.r#type(),
            parent_accessible,
            member_paths,
        );
    }
}

/// Collect member paths where the next path element is an array.
fn collect_array_member_paths<'c: 'a, 'a>(
    root: &impl OperationLike<'c, 'a>,
    prefix: &str,
    array_type: ArrayType<'c>,
    parent_accessible: bool,
    member_paths: &mut HashMap<String, bool>,
) {
    // We're not going to bounds check accesses here, so the prefix is mostly
    // for debugging purposes. We technically don't need to include the brackets
    // but it looks nicer in the diagnostics.
    let num_dims = usize::try_from(array_type.num_dims()).expect("unexpected number of dimensions");
    let indexed_prefix = format!("{prefix}{}", "[]".repeat(num_dims));
    collect_member_paths(
        root,
        &indexed_prefix,
        array_type.element_type(),
        parent_accessible,
        member_paths,
    );
}

/// Convert a FlatSymbolRefAttribute to a String with normalization.
fn flat_symbol_name(attribute: FlatSymbolRefAttribute<'_>) -> String {
    attribute.to_string().trim_start_matches('@').to_string()
}

fn containing_struct_target<'c: 'a, 'a>(operation: &impl OperationLike<'c, 'a>) -> Option<String> {
    let mut opt_parent = get_parent(operation);
    while let Some(parent) = opt_parent {
        if is_struct_def(&parent)
            && let Some(struct_name) = string_attribute(&parent, "sym_name")
        {
            let target = struct_contract_target_name(&parent, &struct_name);
            return Some(target);
        }
        opt_parent = get_parent(&parent);
    }
    None
}

fn containing_template_op<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
) -> Option<llzk::prelude::TemplateOpRef<'c, 'a>> {
    let mut opt_parent = get_parent(operation);
    while let Some(parent) = opt_parent {
        if let Some(template) = llzk::prelude::TemplateOpRef::from_option_raw(parent.to_raw()) {
            return Some(template);
        }
        opt_parent = get_parent(&parent);
    }
    None
}

/// Returns the canonical fully qualified name for a struct contract target.
fn struct_contract_target_name<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
    symbol_name: &str,
) -> String {
    // TODO: We use this path because of a current todo! in llzk-rs in
    // get_fully_qualified_name().
    let mut components = symbol_ancestor_names(operation);
    if components.is_empty() {
        symbol_name.to_string()
    } else {
        components.push(symbol_name.to_string());
        components.join("::")
    }
}

/// Returns the canonical name for a function contract target.
fn function_contract_target_name<'c: 'a, 'a>(
    operation: &impl OperationLike<'c, 'a>,
    symbol_name: &str,
) -> String {
    if let Some(func_op) = FuncDefOpRefMut::from_option_raw(operation.to_raw()) {
        func_op.fully_qualified_name().to_string()
    } else {
        symbol_name.to_string()
    }
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
