//! Base type checker type with shared helpers.

use crate::{
    ast::{Expression, Spanned},
    type_analysis::{TypeInferenceCtx, TypeSystem, helpers::Diagnostics},
};

/// Helper base type for type checkers that implement shared methods.
///
/// Final type-checkers should have a member of this type and [`Deref`] and [`DerefMut`] it to have
/// a more idiomatic API.
pub(super) struct BaseTypeChecker<'ctx, 'ast, T: TypeSystem> {
    pub(super) source_name: &'ast str,
    pub(super) ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
}

impl<'ctx, 'ast, T: TypeSystem> BaseTypeChecker<'ctx, 'ast, T> {
    /// Creates a new base type checker.
    pub fn new(ctx: &'ctx mut TypeInferenceCtx<'ast, T>, source_name: &'ast str) -> Self {
        Self { ctx, source_name }
    }

    /// Helper for running unification and collecting results in the correct format.
    pub fn unify<M: std::fmt::Display>(
        &mut self,
        span: &dyn Spanned,
        context: impl Fn() -> M,
        diags: &mut Diagnostics,
    ) {
        self.ctx
            .unify()
            .err()
            .into_iter()
            .flatten()
            .for_each(|err| diags.add_type_err_at_location(err, context(), span))
    }

    /// Helper for constraining a potential expression to a Felt type.
    pub fn constrain_to_felt(&mut self, expr: Option<&Expression<'ast, T::Type>>) {
        let felt_type = self.ctx.ts().felt_type();
        self.constrain_to(expr, felt_type);
    }

    /// Helper for constraining a potential expression to a Bool type.
    pub fn constrain_to_bool(&mut self, expr: Option<&Expression<'ast, T::Type>>) {
        let bool_type = self.ctx.ts().bool_type();
        self.constrain_to(expr, bool_type);
    }

    /// Helper for constraining a potential expression to a type.
    pub fn constrain_to(&mut self, expr: Option<&Expression<'ast, T::Type>>, expected: T::Type) {
        if let Some(e) = expr {
            self.ctx.add_constraint(expected, e.r#type());
        }
    }

    /// Helper for constraining two potential expressions to have the same type.
    pub fn constrain_equal(
        &mut self,
        lhs: Option<&Expression<'ast, T::Type>>,
        rhs: Option<&Expression<'ast, T::Type>>,
    ) {
        if let Some(lhs) = lhs {
            self.constrain_to(rhs, lhs.r#type())
        }
    }
}
