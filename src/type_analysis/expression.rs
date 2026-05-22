use crate::{
    ast::{
        BinaryOp, Expression, QuantifierDomain,
        QuantifierKind::{Exists, Forall},
        Span, Spanned as _, UnaryOp, Visitable as _, Visitor,
    },
    diagnostic::Diagnostic,
    type_analysis::{
        FnTypeProperties, TypeSystem, TypingResult, ctx::TypeInferenceCtx, helpers::extract_result,
    },
};

pub(super) struct ExpressionTypeCheckerCfg {
    /// Whether the `old` expression is allowed.
    pub allows_old: bool,
}

impl Default for ExpressionTypeCheckerCfg {
    fn default() -> Self {
        Self { allows_old: false }
    }
}

/// Configurable type checker of expressions.
pub(super) struct ExpressionTypeChecker<'ctx, 'ast, T: TypeSystem> {
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
    cfg: ExpressionTypeCheckerCfg,
}

impl<'ctx, 'ast, T: TypeSystem> ExpressionTypeChecker<'ctx, 'ast, T> {
    /// Creates a new expression type checker with default configuration.
    pub fn new(source_name: &'ast str, ctx: &'ctx mut TypeInferenceCtx<'ast, T>) -> Self {
        Self::new_with_cfg(source_name, ctx, Default::default())
    }

    /// Creates a new expression type checker.
    pub fn new_with_cfg(
        source_name: &'ast str,
        ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
        cfg: ExpressionTypeCheckerCfg,
    ) -> Self {
        Self {
            source_name,
            ctx,
            cfg,
        }
    }

    /// Type-checks a binary expression.
    fn check_binary_op_types(
        &mut self,
        op: BinaryOp,
        left: Option<&Expression<'_, T::Type>>,
        right: Option<&Expression<'_, T::Type>>,
        diags: &mut Vec<Diagnostic>,
        span: Span,
    ) {
        let expected = op.expected_type(self.ctx.ts());

        for expr in [left, right].into_iter().flatten() {
            self.ctx.add_constraint(expected.clone(), expr.r#type());
        }

        extract_result(
            self.ctx.unify().map_err(|errs| {
                errs.into_iter()
                    .flat_map(|err| {
                        err.into_diags(self.source_name, Some(span), format!("in binary op '{op}'"))
                    })
                    .collect()
            }),
            diags,
        );
    }

    /// Type-checks an unary expression.
    fn check_unary_op_types(
        &mut self,
        op: UnaryOp,
        expr: Option<&Expression<'_, T::Type>>,
        diags: &mut Vec<Diagnostic>,
        span: Span,
    ) {
        if let Some(expr) = expr {
            let expected = op.expected_type(self.ctx.ts());
            self.ctx.add_constraint(expected, expr.r#type());
        }
        extract_result(
            self.ctx.unify().map_err(|errs| {
                errs.into_iter()
                    .flat_map(|err| {
                        err.into_diags(self.source_name, Some(span), format!("in unary op '{op}'"))
                    })
                    .collect()
            }),
            diags,
        );
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Expression<'ast>> for ExpressionTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Expression<'ast, T::Type>>;

    fn visit(&mut self, expr: &Expression<'ast>) -> Self::Output {
        match expr {
            //  c : Bool, e_0 : t, e_1 : t
            // ----------------------------
            //      c ? e_0 : e_1  : t
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                span,
                ..
            } => {
                let mut diags = vec![];
                let condition = extract_result(condition.accept(self), &mut diags);
                let then_branch = extract_result(then_branch.accept(self), &mut diags);
                let else_branch = extract_result(else_branch.accept(self), &mut diags);

                let bool_type = self.ctx.ts().bool_type();
                if let Some(condition) = &condition {
                    self.ctx.add_constraint(bool_type, condition.r#type());
                }

                if let Some((then_branch, else_branch)) =
                    then_branch.as_ref().zip(else_branch.as_ref())
                {
                    self.ctx
                        .add_constraint(then_branch.r#type(), else_branch.r#type());
                }

                extract_result(
                    self.ctx.unify().map_err(|err| {
                        err.into_iter()
                            .flat_map(|err| {
                                err.into_diags(
                                    self.source_name,
                                    Some(*span),
                                    "on conditional expression",
                                )
                            })
                            .collect()
                    }),
                    &mut diags,
                );

                if !diags.is_empty() {
                    return Err(diags);
                }

                let then_branch = then_branch.unwrap();
                let return_type = self.ctx.resolve(then_branch.r#type());

                Ok(Expression::Conditional {
                    condition: Box::new(condition.unwrap()),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch.unwrap()),
                    span: *span,
                    meta: return_type,
                })
            }

            //  e_0 : Felt, e_1 : Felt, op in {+, -, *, /, %, **, &}
            // ------------------------------------------------------
            //               e_0 (op) e_1 : Felt
            //
            //  e_0 : Felt, e_1 : Felt, op in {<, <=, >, >=, ==, =!}
            // ------------------------------------------------------
            //               e_0 (op) e_1 : Bool
            //
            //  e_0 : Bool, e_1 : Bool, op in {&&, ||, ==, =!}
            // ------------------------------------------------------
            //               e_0 (op) e_1 : Bool
            Expression::Binary {
                op,
                left,
                right,
                span,
                ..
            } => {
                let mut diags = vec![];
                let left = extract_result(left.accept(self), &mut diags);
                let right = extract_result(right.accept(self), &mut diags);
                self.check_binary_op_types(*op, left.as_ref(), right.as_ref(), &mut diags, *span);
                if !diags.is_empty() {
                    return Err(diags);
                }

                Ok(Expression::Binary {
                    op: *op,
                    left: Box::new(left.unwrap()),
                    right: Box::new(right.unwrap()),
                    span: *span,
                    meta: op.return_type(self.ctx.ts()),
                })
            }

            //   e : Felt     e : Bool
            // -----------  -----------
            //  -e : Felt    !e : Bool
            Expression::Unary { op, expr, span, .. } => {
                let mut diags = vec![];
                let expr = extract_result(expr.accept(self), &mut diags);
                self.check_unary_op_types(*op, expr.as_ref(), &mut diags, *span);
                if !diags.is_empty() {
                    return Err(diags);
                }

                Ok(Expression::Unary {
                    op: *op,
                    expr: Box::new(expr.unwrap()),
                    span: *span,
                    meta: op.return_type(self.ctx.ts()),
                })
            }
            Expression::Index { .. } => todo!(),
            Expression::Member { .. } => todo!(),

            //  env.predicates(f) : t_0 * ... * t_k -> t_r, e_0 : t_0, ..., e_k : t_k
            // -----------------------------------------------------------------------
            //                     f(e_0, ..., e_k) : t_r
            //
            // Calls only support predicates for the moment.
            Expression::Call {
                callee, args, span, ..
            } => {
                let bool_type = self.ctx.ts().bool_type();
                let mut diags = vec![];
                // Process arguments.
                let mut new_args = Vec::with_capacity(args.len());
                for arg in args {
                    let arg = extract_result(arg.accept(self), &mut diags);
                    new_args.push(arg);
                }

                // Locate callee.
                let callee_type = self
                    .ctx
                    .scope()
                    .find_predicate(callee)
                    .map_err(|err| {
                        err.into_diags(self.source_name, Some(*span), "on call expression")
                    })?
                    .clone();
                // Add constraints between the types of the expressions and the declared type of
                // the function type.
                let callee_inputs = callee_type.inputs();
                if callee_inputs.len() != new_args.len() {
                    diags.push(Diagnostic::new(
                        self.source_name,
                        format!(
                            "predicate '{}' expects {} arguments but for {}",
                            callee.value(),
                            callee_type.inputs().len(),
                            new_args.len()
                        ),
                        Some(*span),
                    ));
                }
                for (formal, arg) in std::iter::zip(callee_inputs, &new_args) {
                    let Some(arg) = arg else {
                        continue;
                    };
                    self.ctx.add_constraint(formal.clone(), arg.r#type());
                }
                if callee_type.outputs().len() != 1 {
                    diags.push(Diagnostic::new(self.source_name, format!("expected predicate '{}' to return a single boolean expression but return {} expressions", callee.value(), callee_type.outputs().len()), Some(*span)));
                }
                self.ctx
                    .add_constraint(bool_type.clone(), callee_type.outputs()[0].clone());

                extract_result(
                    self.ctx.unify().map_err(|err| {
                        err.into_iter()
                            .flat_map(|err| {
                                err.into_diags(
                                    self.source_name,
                                    Some(*span),
                                    format!("on callsite to '{}'", callee.value()),
                                )
                            })
                            .collect()
                    }),
                    &mut diags,
                );

                if !diags.is_empty() {
                    return Err(diags);
                }

                Ok(Expression::Call {
                    callee: callee.with_meta(callee_type.into()),
                    // If diags is empty then these should be Some.
                    args: new_args.into_iter().map(Option::unwrap).collect(),
                    span: *span,
                    meta: bool_type,
                })
            }

            //       s: Felt, e: Bool
            // -------------------------------
            //   forall s in N..M, e  : Bool
            Expression::Quantifier {
                quantifier_kind: Forall,
                domain: QuantifierDomain::Range { .. },
                ..
            } => todo!(),
            //  e_0 : Array of t, s : t, e_1: Bool
            // ------------------------------------
            //     forall s in e_0, e_1  : Bool
            Expression::Quantifier {
                quantifier_kind: Forall,
                domain: QuantifierDomain::Expr(_),
                ..
            } => todo!(),
            //       s: Felt, e: Bool
            // -------------------------------
            //   exists s in N..M, e  : Bool
            Expression::Quantifier {
                quantifier_kind: Exists,
                domain: QuantifierDomain::Range { .. },
                ..
            } => todo!(),
            //  e_0 : Array of t, s : t, e_1: Bool
            // ------------------------------------
            //     exists s in e_0, e_1  : Bool
            Expression::Quantifier {
                quantifier_kind: Exists,
                domain: QuantifierDomain::Expr(_),
                ..
            } => todo!(),

            //  e_0 : Array of t
            // ------------------
            //  len(e_0) : Felt
            Expression::Len { .. } => todo!(),

            //    e : t
            // -----------
            //  old(e): t
            Expression::Old { span, .. } if !self.cfg.allows_old => Err(vec![Diagnostic::new(
                self.source_name,
                "old expression is not allowed in this context",
                Some(*span),
            )]),
            Expression::Old {
                expression, span, ..
            } => {
                let expression = expression.accept(self)?;
                let meta = expression.r#type();
                Ok(Expression::Old {
                    expression: Box::new(expression),
                    span: *span,
                    meta,
                })
            }

            //  env.parameters(n) : t
            // -----------------------
            //       arg(n) : t
            Expression::Arg { index, span, .. } => {
                let t = self
                    .ctx
                    .scope()
                    .find_parameter(index)
                    .cloned()
                    .map_err(|err| {
                        err.into_diags(
                            self.source_name,
                            Some(*span),
                            format!("on argument #{index}"),
                        )
                    })?;
                Ok(Expression::Arg {
                    index: *index,
                    span: *span,
                    meta: t,
                })
            }

            // ---------------
            //  nondet : Felt
            Expression::Nondet { span, .. } => Ok(Expression::Nondet {
                span: *span,
                meta: self.ctx.ts().felt_type(),
            }),

            // -------------
            //  true : Bool
            //
            // --------------
            //  false : Bool
            Expression::Boolean { value, span, .. } => Ok(Expression::Boolean {
                value: *value,
                span: *span,
                meta: self.ctx.ts().bool_type(),
            }),

            //  0 <= N < P
            // ------------
            //   N : Felt
            Expression::Number { value, span, .. } => Ok(Expression::Number {
                value: *value,
                span: *span,
                meta: self.ctx.ts().felt_type(),
            }),

            //  env.locals(s) : t
            // -------------------
            //       s : t
            Expression::Symbol(ident) => {
                let t = self.ctx.scope().find_local(ident).cloned().map_err(|err| {
                    err.into_diags(
                        self.source_name,
                        Some(ident.span()),
                        format!("on symbol '{}'", ident.value()),
                    )
                })?;

                Ok(Expression::Symbol(ident.with_meta(self.ctx.resolve(t))))
            }
        }
    }
}
