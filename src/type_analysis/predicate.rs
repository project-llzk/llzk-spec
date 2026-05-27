use std::ops::{Deref, DerefMut};

use crate::{
    ast::{Block, PredicateDecl, Spanned, Statement, Visitable as _, Visitor},
    type_analysis::{
        TypeProperties, TypeSystem, TypingResult,
        base::BaseTypeChecker,
        block::{BlockTypeChecker, BlockTypeCheckerCfg},
        ctx::TypeInferenceCtx,
        helpers::Diagnostics,
    },
};

/// Handles type checking predicate declarations.
pub(super) struct PredicateTypeChecker<'ctx, 'ast, T: TypeSystem> {
    base: BaseTypeChecker<'ctx, 'ast, T>,
}

impl<'ctx, 'ast, T: TypeSystem> PredicateTypeChecker<'ctx, 'ast, T> {
    /// Creates a new predicate type checker.
    pub fn new(source_name: &'ast str, ctx: &'ctx mut TypeInferenceCtx<'ast, T>) -> Self {
        Self {
            base: BaseTypeChecker::new(ctx, source_name),
        }
    }

    /// Ensures that, with the exception of the last statement, the body of the predicate does not
    /// contain `return` statements.
    fn ensure_no_early_return(&self, block: &Block<'_, T::Type>, diags: &mut Diagnostics) {
        block
            .statements()
            .iter()
            .rev()
            // Skip the last statement since that one is allowed to be a return.
            .skip(1)
            .rev()
            .filter(|stmt| matches!(stmt, Statement::Return { .. }))
            .for_each(|stmt| {
                diags.add_at_location(
                    "return statements must be the last statement in a predicate",
                    stmt,
                )
            })
    }

    /// Ensures that the body of the predicate ends with a return statement and that the returned
    /// expression has a boolean type.
    ///
    /// The type check is performed via a type constraint so this method should be called before
    /// [`Self::ensure_full_param_monomorphization`] to allow propagating types. Otherwise
    /// predicates like `predicate foo(x) = x` will fails to type check.
    fn ensure_return_terminator(&mut self, block: &Block<'_, T::Type>, diags: &mut Diagnostics) {
        let Some(last) = block.statements().last() else {
            diags.add("predicates cannot have an empty body");
            return;
        };

        match last {
            Statement::Return { expression, span } => {
                let bool_type = self.ctx.ts().bool_type();
                self.ctx.add_constraint(bool_type, expression.r#type());
                self.unify(span, || "on return statement", diags);
            }
            _ => diags.add("predicates with a block body must end with a return statement"),
        }
    }

    /// Generate N fresh type variables
    fn fresh_vars(&mut self, n: usize) -> Vec<T::Type> {
        std::iter::repeat_with(|| self.ctx.ts().fresh_var())
            .take(n)
            .collect()
    }

    /// Helper function that binds the symbols in the correct scopes and pushes into a new scope.
    fn bind_and_push(
        &mut self,
        decl: &PredicateDecl<'ast>,
        diags: &mut Diagnostics,
    ) -> Vec<T::Type> {
        let ins = self.fresh_vars(decl.params().len());
        let fn_type = self.ctx.ts().predicate_type(&ins);

        // Bind the predicate to the current scope.
        diags.extract_type_result(
            self.ctx.scope().top().bind_predicate(decl.name(), fn_type),
            || format!("on declaration of predicate '{}'", decl.name().value()),
        );
        // Push a local limit scope to avoid inner predicates accessing outer locals.
        self.ctx.scope().push_local_limit(());
        // Bind the formals to the new scope.
        for (formal, r#type) in std::iter::zip(decl.params(), &ins) {
            diags.extract_type_result(
                self.ctx.scope().top().bind_local(formal, r#type.clone()),
                || {
                    format!(
                        "on parameter '{}' of predicate '{}'",
                        formal.value(),
                        decl.name().value()
                    )
                },
            );
        }

        ins
    }

    /// Ensures that all parameters of the predicate have been assigned a concrete type.
    ///
    /// # Panics
    ///
    /// Call this method BEFORE popping the scope used for checking the body. If done after, the
    /// parameters will disappear and this method will panic.
    fn ensure_full_param_monomorphization(
        &mut self,
        decl: &PredicateDecl<'ast>,
        diags: &mut Diagnostics,
    ) {
        for name in decl.params() {
            // The param MUST exist. If it doesn't is a bug in our part because we are probably
            // calling this method wrong.
            diags.add_unless(
                !self.ctx.scope().find_local(name).unwrap().is_var_type(),
                || {
                    format!(
                        "parameter '{}' in predicate '{}' has an ambigous type",
                        name.value(),
                        decl.name().value()
                    )
                },
            );
        }
    }

    /// Configuration for type-checking inside a predicate block.
    fn block_cfg() -> BlockTypeCheckerCfg {
        BlockTypeCheckerCfg {
            allows_invariants: false,
            allows_scoped: false,
            allows_ensure_and_require: false,
            allows_return: true,
            allows_invariant_stmts: false,
            allows_arg: false,
        }
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<PredicateDecl<'ast>>
    for PredicateTypeChecker<'ctx, 'ast, T>
{
    type Output = TypingResult<PredicateDecl<'ast, T::Type>>;

    fn visit(&mut self, decl: &PredicateDecl<'ast>) -> Self::Output {
        let mut diags = Diagnostics::new(self.source_name, decl);
        let ins = self.bind_and_push(decl, &mut diags);

        let mut block_tc = BlockTypeChecker::new(self.source_name, self.ctx, Self::block_cfg());
        let body = diags.extract_result(decl.body().accept(&mut block_tc));

        if let Some(body) = body.as_ref() {
            // Check that the last statement is a return and no other statement is.
            self.ensure_no_early_return(body, &mut diags);
            self.ensure_return_terminator(body, &mut diags);
            // Check that all parameters have been assigned a type (i.e. they are used within the body
            // of the predicate and thus been monomorphized).
            self.ensure_full_param_monomorphization(decl, &mut diags);
        }
        self.ctx.scope().pop();

        diags.finish(move || {
            create_decl(
                decl,
                ins,
                // Fetch the prediate's type again to get the update version post-inference.
                self.ctx
                    .scope()
                    .find_predicate(decl.name())
                    .unwrap()
                    .clone(),
                body.unwrap(),
            )
        })
    }
}

fn create_decl<'ast, T>(
    decl: &PredicateDecl<'ast>,
    ins: Vec<T>,
    fn_type: impl Into<T>,
    body: Block<'ast, T>,
) -> PredicateDecl<'ast, T> {
    let params = std::iter::zip(decl.params(), ins)
        .map(|(ident, r#type)| ident.with_meta(r#type))
        .collect::<Vec<_>>();
    PredicateDecl::new(
        decl.name().with_meta(fn_type.into()),
        params,
        body,
        decl.span(),
    )
}

impl<'ctx, 'ast, T: TypeSystem> DerefMut for PredicateTypeChecker<'ctx, 'ast, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<'ctx, 'ast, T: TypeSystem> Deref for PredicateTypeChecker<'ctx, 'ast, T> {
    type Target = BaseTypeChecker<'ctx, 'ast, T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
