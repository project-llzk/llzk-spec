use crate::{
    ast::{Block, PredicateDecl, Spanned, Statement, Visitable as _, Visitor},
    diagnostic::Diagnostic,
    type_analysis::{
        TypeProperties, TypeSystem, TypingResult,
        block::{BlockTypeChecker, BlockTypeCheckerCfg},
        ctx::TypeInferenceCtx,
        helpers::extract_result,
    },
};

/// Handles type checking predicate declarations.
pub(super) struct PredicateTypeChecker<'ctx, 'ast, T: TypeSystem> {
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
}

impl<'ctx, 'ast, T: TypeSystem> PredicateTypeChecker<'ctx, 'ast, T> {
    /// Creates a new predicate type checker.
    pub fn new(ctx: &'ctx mut TypeInferenceCtx<'ast, T>, source_name: &'ast str) -> Self {
        Self { ctx, source_name }
    }

    /// Ensures that, with the exception of the last statement, the body of the predicate does not
    /// contain `return` statements.
    fn ensure_no_early_return(&self, block: &Block<'_, T::Type>, diags: &mut Vec<Diagnostic>) {
        diags.extend(
            block
                .statements()
                .iter()
                .rev()
                // Skip the last statement since that one is allowed to be a return.
                .skip(1)
                .rev()
                .filter(|stmt| matches!(stmt, Statement::Return { .. }))
                .map(|stmt| {
                    Diagnostic::new(
                        self.source_name,
                        "return statements must be the last statement in a predicate",
                        Some(stmt.span()),
                    )
                }),
        )
    }

    /// Ensures that the body of the predicate ends with a return statement and that the returned
    /// expression has a boolean type.
    ///
    /// The type check is performed via a type constraint so this method should be called before
    /// [`Self::ensure_full_param_monomorphization`] to allow propagating types. Otherwise
    /// predicates like `predicate foo(x) = x` will fails to type check.
    fn ensure_return_terminator(
        &mut self,
        block: &Block<'_, T::Type>,
        diags: &mut Vec<Diagnostic>,
    ) {
        let Some(last) = block.statements().last() else {
            diags.push(Diagnostic::new(
                self.source_name,
                "predicates cannot have an empty body",
                Some(block.span()),
            ));
            return;
        };

        match last {
            Statement::Return { expression, span } => {
                let bool_type = self.ctx.ts().bool_type();
                self.ctx.add_constraint(bool_type, expression.r#type());
                extract_result(
                    self.ctx.unify().map_err(|err| {
                        err.into_iter()
                            .flat_map(|err| {
                                err.into_diags(self.source_name, Some(*span), "on return statement")
                            })
                            .collect()
                    }),
                    diags,
                );
            }
            _ => diags.push(Diagnostic::new(
                self.source_name,
                "predicates with a block body must end with a return statement",
                Some(block.span()),
            )),
        }
    }

    /// Helper function that binds the symbols in the correct scopes and pushes into a new scope.
    fn bind_and_push(
        &mut self,
        decl: &PredicateDecl<'ast>,
        diags: &mut Vec<Diagnostic>,
    ) -> Vec<T::Type> {
        let ins = Vec::from_iter(
            std::iter::repeat_with(|| self.ctx.ts().fresh_var()).take(decl.params().len()),
        );
        let fn_type = self.ctx.ts().predicate_type(&ins);

        // Bind the predicate to the current scope.
        extract_result(
            self.ctx
                .scope()
                .top()
                .bind_predicate(decl.name(), fn_type)
                .map_err(|err| {
                    err.into_diags(
                        self.source_name,
                        Some(decl.span()),
                        format!("on declaration of predicate '{}'", decl.name().value()),
                    )
                }),
            diags,
        );
        // Push a local limit scope to avoid inner predicates accessing outer locals.
        self.ctx.scope().push_local_limit(());
        // Bind the formals to the new scope.
        for (formal_no, (formal, r#type)) in std::iter::zip(decl.params(), &ins).enumerate() {
            extract_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_parameter(formal, r#type.clone(), formal_no)
                    .map_err(|err| {
                        err.into_diags(
                            self.source_name,
                            Some(decl.span()),
                            format!(
                                "on parameter #{formal_no} '{}' of predicate '{}'",
                                formal.value(),
                                decl.name().value()
                            ),
                        )
                    }),
                diags,
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
        diags: &mut Vec<Diagnostic>,
    ) {
        for name in decl.params() {
            // The param MUST exist. If it doesn't is a bug in our part because we are probably
            // calling this method wrong.
            let binding = self.ctx.scope().find_local(name).unwrap();
            if binding.is_var_type() {
                diags.push(Diagnostic::new(
                    self.source_name,
                    format!(
                        "parameter '{}' in predicate '{}' has an ambigous type",
                        name.value(),
                        decl.name().value()
                    ),
                    Some(name.span()),
                ));
            }
        }
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<PredicateDecl<'ast>>
    for PredicateTypeChecker<'ctx, 'ast, T>
{
    type Output = TypingResult<PredicateDecl<'ast, T::Type>>;

    fn visit(&mut self, decl: &PredicateDecl<'ast>) -> Self::Output {
        let mut diags = vec![];
        let ins = self.bind_and_push(decl, &mut diags);

        let mut block_tc = BlockTypeChecker::new(
            self.source_name,
            self.ctx,
            BlockTypeCheckerCfg {
                allows_invariants: false,
                allows_scoped: false,
            },
        );
        let body = decl.body().accept(&mut block_tc)?;

        // Check that the last statement is a return and no other statement is.
        self.ensure_no_early_return(&body, &mut diags);
        self.ensure_return_terminator(&body, &mut diags);
        // Check that all parameters have been assigned a type (i.e. they are used within the body
        // of the predicate and thus been monomorphized).
        self.ensure_full_param_monomorphization(decl, &mut diags);
        self.ctx.scope().pop();

        if !diags.is_empty() {
            return Err(diags);
        }
        Ok(create_decl(
            decl,
            ins,
            // Fetch the prediate's type again to get the update version post-inference.
            self.ctx
                .scope()
                .find_predicate(decl.name())
                .unwrap()
                .clone(),
            body,
        ))
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
