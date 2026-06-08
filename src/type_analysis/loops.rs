//! Types for working with information about loops defined inside circuits.

use crate::{
    ast::{AstContext, Symbol},
    type_analysis::TypeSystem,
};

/// Information about a loop.
#[derive(Debug)]
pub struct LoopInfo<T> {
    /// Label given to the loop.
    label: LoopLabel,
    /// Bindings the loop defines.
    bindings: Vec<(LoopBinding, T)>,
}

impl<T> LoopInfo<T> {
    /// Creates information about a for loop.
    ///
    /// `args` must be any additional arguments that are passed to the loop.
    ///
    /// The binding's content (`T`) for the intrinsic arguments is obtained by calling the factory
    /// callback for each one.
    pub fn new_for_loop(
        label: LoopLabel,
        mut f: impl FnMut(LoopBinding) -> T,
        args: impl IntoIterator<Item = T>,
    ) -> Self {
        use LoopBinding::*;
        Self {
            label,
            bindings: Vec::from_iter(
                [(Lb, f(Lb)), (Iv, f(Iv)), (Ub, f(Ub)), (Step, f(Step))]
                    .into_iter()
                    .chain(args.into_iter().enumerate().map(|(n, arg)| (Arg(n), arg))),
            ),
        }
    }

    /// Creates information about a while loop.
    pub fn new_while_loop(label: LoopLabel, args: impl IntoIterator<Item = T>) -> Self {
        Self {
            label,
            bindings: Vec::from_iter(
                args.into_iter()
                    .enumerate()
                    .map(|(n, arg)| (LoopBinding::Arg(n), arg)),
            ),
        }
    }

    /// Returns the bindings defined by the loop.
    pub(super) fn bindings(&self) -> &[(LoopBinding, T)] {
        &self.bindings
    }

    /// Symbolizes the label of the loop.
    pub fn symbolize_label<'ast>(&self, ast: &'ast AstContext) -> Symbol<'ast> {
        ast.new_symbol(self.label.to_string())
    }
}

/// Implementation of the loop label.
///
/// Is kept hidden to force clients to use the constructors in [`LoopLabel`] instead of
/// the enum's constructors.
#[derive(Debug, Clone)]
enum LoopLabelImpl {
    Named(String),
    Indexed(usize),
}

/// Label given to a loop by the circuit.
///
/// Can be an explicit label or it can be implicitly derived from the loop's position in the
/// circuit.
///
/// The naming convention for loop labels is handled by this type and clients should not attempt to
/// give names to loops that are not explicitly labeled.
#[derive(Clone)]
pub struct LoopLabel {
    inner: LoopLabelImpl,
}

impl LoopLabel {
    /// Creates a named label.
    pub fn named(name: &str) -> Self {
        Self {
            inner: LoopLabelImpl::Named(name.to_owned()),
        }
    }

    /// Creates an implicit label.
    pub fn implicit(index: usize) -> Self {
        Self {
            inner: LoopLabelImpl::Indexed(index),
        }
    }
}

impl std::fmt::Debug for LoopLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}

impl std::fmt::Display for LoopLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            LoopLabelImpl::Named(name) => write!(f, "{name}"),
            LoopLabelImpl::Indexed(idx) => write!(f, "loop{idx}"),
        }
    }
}

/// Names for the different bindings a loop can define.
///
/// Invariant bindings expose the loop values to the invariant body. For `scf.for`, the
/// binding order is lower bound, induction variable, upper bound, step, then iter args.
/// For `scf.while`, bindings are the loop-carried block arguments in order.
#[derive(Debug)]
pub enum LoopBinding {
    /// Lower bound. Only present in for loops.
    Lb,
    /// Induction variable.
    Iv,
    /// Upper bound. Only present in for loops.
    Ub,
    /// Step. Only present in for loops.
    Step,
    /// Additional arguments. Depending on the type of loop the n-th argument will have a different
    /// physical position in the bindings list.
    Arg(usize),
}
