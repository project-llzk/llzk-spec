use crate::{
    ast::{
        BinaryOp, Block, Expression, Span, Spanned as _, Statement, UnaryOp, Visitable as _,
        Visitor,
    },
    diagnostic::Diagnostic,
    type_analysis::{
        FnTypeProperties, TypeSystem, TypingResult,
        ctx::TypeInferenceCtx,
        helpers::{check_many, extract_result},
        predicate::PredicateTypeChecker,
    },
};

pub(super) struct BlockTypeChecker<'ctx, 'ast, T: TypeSystem> {
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
    /// Whether invariant declarations are allowed in this context.
    allows_invariants: bool,
    /// Whether scoped blocks are allowed in this context.
    allows_scoped: bool,
}

impl<'ctx, 'ast, T: TypeSystem> BlockTypeChecker<'ctx, 'ast, T> {
    pub fn new(
        source_name: &'ast str,
        ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
        allows_invariants: bool,
        allows_scoped: bool,
    ) -> Self {
        Self {
            source_name,
            ctx,
            allows_invariants,
            allows_scoped,
        }
    }

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

impl<'ast, 'ctx, T: TypeSystem> Visitor<Block<'ast>> for BlockTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Block<'ast, T::Type>>;

    fn visit(&mut self, block: &Block<'ast>) -> Self::Output {
        check_many(self, block, |statements| {
            Block::new(statements, block.span())
        })
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Statement<'ast>> for BlockTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Statement<'ast, T::Type>>;

    fn visit(&mut self, statement: &Statement<'ast>) -> Self::Output {
        match statement {
            Statement::Scoped {
                scope,
                statement,
                span,
            } => {
                let result = statement.accept(self);

                if self.allows_scoped {
                    result.map(|inner| Statement::Scoped {
                        scope: *scope,
                        statement: Box::new(inner),
                        span: *span,
                    })
                } else {
                    let mut diags = result.err().unwrap_or_default();
                    diags.push(Diagnostic::new(
                        self.source_name,
                        "scoped block is not allowed in this context",
                        Some(*span),
                    ));
                    Err(diags)
                }
            }
            Statement::Block(block) => {
                self.ctx.scope().push();
                let new = block.accept(self).map(Statement::Block);
                self.ctx.scope().pop();
                new
            }
            Statement::Require { expression, span } => Ok(Statement::Require {
                expression: expression.accept(self)?,
                span: *span,
            }),
            Statement::Ensure { expression, span } => Ok(Statement::Ensure {
                expression: expression.accept(self)?,
                span: *span,
            }),
            Statement::Let { name, value, span } => {
                let value = value.accept(self)?;
                self.ctx
                    .scope()
                    .top()
                    .bind_local(name, value.r#type())
                    .map_err(|err| {
                        err.into_diags(self.source_name, Some(*span), "in let statement")
                    })?;
                Ok(Statement::Let {
                    name: name.with_meta(value.r#type()),
                    value,
                    span: *span,
                })
            }
            // TODO: We need to know the type of the unused identifier.
            Statement::Unused { .. } => todo!(),
            Statement::Return { expression, span } => Ok(Statement::Return {
                expression: expression.accept(self)?,
                span: *span,
            }),
            Statement::Increases { expression, span } => Ok(Statement::Increases {
                expression: expression.accept(self)?,
                span: *span,
            }),
            Statement::Decreases { expression, span } => Ok(Statement::Decreases {
                expression: expression.accept(self)?,
                span: *span,
            }),
            Statement::Step { expression, span } => Ok(Statement::Step {
                expression: expression.accept(self)?,
                span: *span,
            }),

            Statement::Invariant(decl) => {
                if self.allows_invariants {
                    todo!()
                } else {
                    Err(vec![Diagnostic::new(
                        self.source_name,
                        "invariant not allowed in this context",
                        Some(decl.span()),
                    )])
                }
            }

            Statement::Predicate(decl) => {
                let mut pred_tc = PredicateTypeChecker::new(self.ctx, self.source_name);
                decl.accept(&mut pred_tc).map(Statement::Predicate)
            }
            Statement::Empty { span } => Ok(Statement::Empty { span: *span }),
        }
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Expression<'ast>> for BlockTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Expression<'ast, T::Type>>;

    fn visit(&mut self, expr: &Expression<'ast>) -> Self::Output {
        match expr {
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
            // Calls only support predicates for the moment.
            Expression::Call {
                callee, args, span, ..
            } => {
                let bool_type = self.ctx.ts().bool_type();
                let mut diags = vec![];
                // Process arguments.
                let mut new_args = vec![];
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
                if callee_type.inputs().len() != new_args.len() {
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
                for (formal, arg) in std::iter::zip(callee_type.inputs(), &new_args) {
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
            Expression::Quantifier { .. } => todo!(),
            Expression::Len { .. } => todo!(),
            Expression::Old { .. } => todo!(),
            Expression::Arg { .. } => todo!(),
            Expression::Nondet { span, .. } => Ok(Expression::Nondet {
                span: *span,
                meta: self.ctx.ts().felt_type(),
            }),
            Expression::Boolean { value, span, .. } => Ok(Expression::Boolean {
                value: *value,
                span: *span,
                meta: self.ctx.ts().bool_type(),
            }),
            Expression::Number { value, span, .. } => Ok(Expression::Number {
                value: *value,
                span: *span,
                meta: self.ctx.ts().felt_type(),
            }),
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
