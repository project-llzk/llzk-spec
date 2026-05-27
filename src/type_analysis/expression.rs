use std::ops::{Deref, DerefMut};

use crate::{
    ast::{Expression, QuantifierDomain, Visitable as _, Visitor},
    type_analysis::{
        FnTypeProperties, TypeSystem, TypingResult, base::BaseTypeChecker, ctx::TypeInferenceCtx,
        helpers::Diagnostics,
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
    base: BaseTypeChecker<'ctx, 'ast, T>,
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
            base: BaseTypeChecker::new(ctx, source_name),
            cfg,
        }
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Expression<'ast>> for ExpressionTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Expression<'ast, T::Type>>;

    fn visit(&mut self, expr: &Expression<'ast>) -> Self::Output {
        let mut diags = Diagnostics::new(self.source_name, expr);
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
                let condition = diags.extract_result(condition.accept(self));
                let then_branch = diags.extract_result(then_branch.accept(self));
                let else_branch = diags.extract_result(else_branch.accept(self));

                self.constraint_to_bool(condition.as_ref());
                self.constraint_equal(then_branch.as_ref(), else_branch.as_ref());
                self.unify(span, || "on conditional expression", &mut diags);

                diags.finish(|| {
                    let then_branch = then_branch.unwrap();
                    let return_type = self.ctx.resolve(then_branch.r#type());
                    Expression::Conditional {
                        condition: Box::new(condition.unwrap()),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch.unwrap()),
                        span: *span,
                        meta: return_type,
                    }
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
                let left = diags.extract_result(left.accept(self));
                let right = diags.extract_result(right.accept(self));
                let expected = op.expected_type(self.ctx.ts());
                self.constraint_to(left.as_ref(), expected.clone());
                self.constraint_to(right.as_ref(), expected);
                self.unify(span, || format!("in binary op '{op}'"), &mut diags);

                diags.finish(|| Expression::Binary {
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
                let expr = diags.extract_result(expr.accept(self));
                let expected = op.expected_type(self.ctx.ts());
                self.constraint_to(expr.as_ref(), expected);
                self.unify(span, || format!("in unary op '{op}'"), &mut diags);

                diags.finish(|| Expression::Unary {
                    op: *op,
                    expr: Box::new(expr.unwrap()),
                    span: *span,
                    meta: op.return_type(self.ctx.ts()),
                })
            }

            //  e_0 : Array of t, e_1 : Felt
            // ------------------------------
            //          e_0[e_1] : t
            Expression::Index {
                target,
                index,
                span,
                ..
            } => {
                let target = diags.extract_result(target.accept(self));
                let index = diags.extract_result(index.accept(self));

                let result_type = self.ctx.ts().fresh_var();
                self.constraint_to_felt(index.as_ref());
                if let Some(target) = target.as_ref() {
                    self.ctx
                        .add_array_constraint(target.r#type(), result_type.clone());
                }

                self.unify(span, || "on index expression", &mut diags);
                diags.finish(|| Expression::Index {
                    target: Box::new(target.unwrap()),
                    index: Box::new(index.unwrap()),
                    span: *span,
                    meta: self.ctx.resolve(result_type),
                })
            }

            //  e : t_0 <: { m : t_1 }
            // ------------------------
            //        e.m : t_1
            //
            // t_0' has a member 'm' whose type is constrained to be 't_1'.
            Expression::Member {
                target,
                member,
                span,
                ..
            } => {
                let target = diags.extract_result(target.accept(self));
                let result_type = self.ctx.ts().fresh_var();
                if let Some(target) = target.as_ref() {
                    self.ctx.add_member_constraint(
                        target.r#type(),
                        member.symbol(),
                        result_type.clone(),
                    );
                }
                self.unify(span, || "on member access expression", &mut diags);
                diags.finish(|| {
                    let result_type = self.ctx.resolve(result_type);
                    Expression::Member {
                        target: Box::new(target.unwrap()),
                        member: member.with_meta(result_type.clone()),
                        span: *span,
                        meta: result_type,
                    }
                })
            }

            //  env.predicates(f) : t_0 * ... * t_k -> t_r, e_0 : t_0, ..., e_k : t_k
            // -----------------------------------------------------------------------
            //                     f(e_0, ..., e_k) : t_r
            //
            // Calls only support predicates for the moment.
            Expression::Call {
                callee, args, span, ..
            } => {
                let bool_type = self.ctx.ts().bool_type();
                // Process arguments.
                let new_args = diags.extract_many_results(args.iter().map(|arg| arg.accept(self)));

                // Locate callee.
                let callee_type = diags.to_typing_result(
                    self.ctx.scope().find_predicate(callee).cloned(),
                    || "on call expression",
                )?;
                // Add constraints between the types of the expressions and the declared type of
                // the function type.
                let callee_inputs = callee_type.inputs();
                diags.add_unless(callee_inputs.len() == new_args.len(), || {
                    format!(
                        "predicate '{}' expects {} arguments but got {}",
                        callee.value(),
                        callee_type.inputs().len(),
                        new_args.len()
                    )
                });

                std::iter::zip(callee_inputs, &new_args).for_each(|(formal, arg)| {
                    self.constraint_to(arg.as_ref(), formal);
                });
                let callee_outputs = callee_type.outputs();
                diags.add_unless(callee_outputs.len() == 1, || format!("expected predicate '{}' to return a single boolean expression but returns {} expressions", callee.value(), callee_outputs.len()));
                self.ctx.add_constraint(
                    bool_type.clone(),
                    callee_outputs.into_iter().next().unwrap(),
                );

                self.unify(
                    span,
                    || format!("on callsite to '{}'", callee.value()),
                    &mut diags,
                );

                diags.finish(|| Expression::Call {
                    callee: callee.with_meta(self.ctx.resolve(callee_type.into())),
                    // If diags is empty then these should be Some.
                    args: new_args.into_iter().flatten().collect(),
                    span: *span,
                    meta: bool_type,
                })
            }

            //       s: Felt, e: Bool
            // -------------------------------
            //   forall s in N..M, e  : Bool
            //
            //       s: Felt, e: Bool
            // -------------------------------
            //   exists s in N..M, e  : Bool
            //
            //  e_0 : Array of t, s : t, e_1: Bool
            // ------------------------------------
            //     forall s in e_0, e_1  : Bool
            //
            //  e_0 : Array of t, s : t, e_1: Bool
            // ------------------------------------
            //     exists s in e_0, e_1  : Bool
            Expression::Quantifier {
                quantifier_kind,
                domain,
                binding,
                body,
                span,
                ..
            } => {
                let (binding_type, domain) = match domain {
                    QuantifierDomain::Range {
                        start,
                        end,
                        span: range_span,
                    } => {
                        // Constraint N and M to be felts.
                        let domain_start = diags.extract_result(start.accept(self));
                        let domain_end = diags.extract_result(end.accept(self));
                        self.constraint_to_felt(domain_start.as_ref());
                        self.constraint_to_felt(domain_end.as_ref());
                        (
                            // The type of the binding is always Felt in this case.
                            self.ctx.ts().felt_type(),
                            domain_start.zip(domain_end).map(|(start, end)| {
                                QuantifierDomain::Range {
                                    start: Box::new(start),
                                    end: Box::new(end),
                                    span: *range_span,
                                }
                            }),
                        )
                    }
                    QuantifierDomain::Expr(expression) => {
                        // Bind the quantifier's local to a fresh variable.
                        let binding_type = self.ctx.ts().fresh_var();
                        let expr = diags.extract_result(expression.accept(self));
                        if let Some(expr) = expr.as_ref() {
                            self.ctx
                                .add_array_constraint(expr.r#type(), binding_type.clone());
                        }
                        (
                            binding_type,
                            expr.map(|expr| QuantifierDomain::Expr(Box::new(expr))),
                        )
                    }
                };
                self.ctx.scope().push(());
                diags.extract_type_result(
                    self.ctx
                        .scope()
                        .top()
                        .bind_local(binding, binding_type.clone()),
                    || "on quantifier expression",
                );
                let body = diags.extract_result(body.accept(self));
                self.ctx.scope().pop();

                self.constraint_to_bool(body.as_ref());
                self.unify(
                    span,
                    || format!("in {quantifier_kind} expression"),
                    &mut diags,
                );

                diags.finish(|| Expression::Quantifier {
                    quantifier_kind: *quantifier_kind,
                    binding: binding.with_meta(self.ctx.resolve(binding_type)),
                    domain: domain.unwrap(),
                    body: Box::new(body.unwrap()),
                    span: *span,
                    meta: self.ctx.ts().bool_type(),
                })
            }

            //  e_0 : Array of t
            // ------------------
            //  len(e_0) : Felt
            Expression::Len { target, span, .. } => {
                let target = diags.extract_result(target.accept(self));
                if let Some(target) = target.as_ref() {
                    let v = self.ctx.ts().fresh_var();
                    self.ctx.add_array_constraint(target.r#type(), v);
                }
                self.unify(span, || "on lenght expression", &mut diags);
                diags.finish(|| Expression::Len {
                    target: Box::new(target.unwrap()),
                    span: *span,
                    meta: self.ctx.ts().felt_type(),
                })
            }

            //    e : t
            // -----------
            //  old(e): t
            Expression::Old {
                expression, span, ..
            } => {
                diags.add_unless(
                    self.cfg.allows_old,
                    || "old expression is not allowed in this context",
                );
                let expression = diags.extract_result(expression.accept(self));
                diags.finish(|| {
                    let expression = expression.unwrap();
                    let meta = expression.r#type();
                    Expression::Old {
                        expression: Box::new(expression),
                        span: *span,
                        meta,
                    }
                })
            }

            //  env.parameters(n) : t
            // -----------------------
            //       arg(n) : t
            Expression::Arg { index, span, .. } => diags
                .to_typing_result(self.ctx.scope().find_parameter(index).cloned(), || {
                    format!("on argument #{index}")
                })
                .map(|t| Expression::Arg {
                    index: *index,
                    span: *span,
                    meta: t,
                }),

            // ---------------
            //  nondet : Felt
            Expression::Nondet { span, .. } => diags.finish(|| Expression::Nondet {
                span: *span,
                meta: self.ctx.ts().felt_type(),
            }),

            // -------------
            //  true : Bool
            //
            // --------------
            //  false : Bool
            Expression::Boolean { value, span, .. } => diags.finish(|| Expression::Boolean {
                value: *value,
                span: *span,
                meta: self.ctx.ts().bool_type(),
            }),

            //  0 <= N < P
            // ------------
            //   N : Felt
            Expression::Number { value, span, .. } => diags.finish(|| Expression::Number {
                value: *value,
                span: *span,
                meta: self.ctx.ts().felt_type(),
            }),

            //  env.locals(s) : t
            // -------------------
            //       s : t
            Expression::Symbol(ident) => diags
                .to_typing_result(self.ctx.scope().find_local(ident).cloned(), || {
                    format!("on symbol '{}'", ident.value())
                })
                .map(|t| Expression::Symbol(ident.with_meta(self.ctx.resolve(t)))),
        }
    }
}

impl<'ctx, 'ast, T: TypeSystem> DerefMut for ExpressionTypeChecker<'ctx, 'ast, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<'ctx, 'ast, T: TypeSystem> Deref for ExpressionTypeChecker<'ctx, 'ast, T> {
    type Target = BaseTypeChecker<'ctx, 'ast, T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
