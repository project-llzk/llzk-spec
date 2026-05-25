use crate::{
    ast::{Block, Spanned as _, Statement, Visitable as _, Visitor},
    diagnostic::Diagnostic,
    type_analysis::{
        TypeSystem, TypingResult,
        ctx::TypeInferenceCtx,
        expression::{ExpressionTypeChecker, ExpressionTypeCheckerCfg},
        helpers::{check_many, extract_result},
        invariant::InvariantTypeChecker,
        predicate::PredicateTypeChecker,
    },
};

pub(super) struct BlockTypeCheckerCfg {
    /// Whether invariant declarations are allowed in this context.
    pub allows_invariants: bool,
    /// Whether scoped blocks are allowed in this context.
    pub allows_scoped: bool,
    /// Whether ensure and require statements are allowed in this context.
    pub allows_ensure_and_require: bool,
    /// Whether return statements are allowed in this context.
    pub allows_return: bool,
    /// Whether increases, decreases, and step statements are allowed in this context.
    pub allows_invariant_stmts: bool,
}

/// Configurable type checker of generic blocks of code.
///
/// Declaration-like AST entities can reuse this type checker by passing the correct configuration
/// for their particular semantics.
pub(super) struct BlockTypeChecker<'ctx, 'ast, T: TypeSystem> {
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
    cfg: BlockTypeCheckerCfg,
}

impl<'ctx, 'ast, T: TypeSystem> BlockTypeChecker<'ctx, 'ast, T> {
    /// Creates a new block type checker.
    pub fn new(
        source_name: &'ast str,
        ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
        cfg: BlockTypeCheckerCfg,
    ) -> Self {
        Self {
            source_name,
            ctx,
            cfg,
        }
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

macro_rules! stmt_not_allowed {
    ($name:literal, $self:expr, $span:expr, $diags:expr) => {{
        let mut diags = $diags;
        diags.push(Diagnostic::new(
            $self.source_name,
            concat!($name, " statement is not allowed in this context"),
            Some(*$span),
        ));
        Err(diags)
    }};
    ($name:literal, $self:expr, $span:expr) => {
        stmt_not_allowed!($name, $self, $span, Vec::<Diagnostic>::with_capacity(1))
    };
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Statement<'ast>> for BlockTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Statement<'ast, T::Type>>;

    fn visit(&mut self, statement: &Statement<'ast>) -> Self::Output {
        match statement {
            Statement::Scoped {
                scope,
                statement,
                span,
            } if !self.cfg.allows_scoped => {
                stmt_not_allowed!(
                    "scoped block",
                    self,
                    span,
                    statement.accept(self).err().unwrap_or_default()
                )
            }
            Statement::Scoped {
                scope,
                statement,
                span,
            } => {
                self.ctx.scope().push(());
                let new = statement.accept(self).map(|inner| Statement::Scoped {
                    scope: *scope,
                    statement: Box::new(inner),
                    span: *span,
                });
                self.ctx.scope().pop();
                new
            }

            Statement::Block(block) => {
                self.ctx.scope().push(());
                let new = block.accept(self).map(Statement::Block);
                self.ctx.scope().pop();
                new
            }

            //    e : Bool
            // ----------------
            //  require e : ()
            Statement::Require { expression, span } if !self.cfg.allows_ensure_and_require => {
                stmt_not_allowed!("require", self, span)
            }
            Statement::Require { expression, span } => {
                let mut diags = vec![];
                let mut expr_tc = ExpressionTypeChecker::new(self.source_name, self.ctx);
                let expression = extract_result(expression.accept(&mut expr_tc), &mut diags);
                if let Some(expr) = expression.as_ref() {
                    let bool_type = self.ctx.ts().bool_type();
                    self.ctx.add_constraint(bool_type, expr.r#type());
                    extract_result(
                        self.ctx.unify().map_err(|errs| {
                            errs.into_iter()
                                .flat_map(|err| {
                                    err.into_diags(
                                        self.source_name,
                                        Some(*span),
                                        "on require statement",
                                    )
                                })
                                .collect()
                        }),
                        &mut diags,
                    );
                }
                if !diags.is_empty() {
                    return Err(diags);
                }
                Ok(Statement::Require {
                    expression: expression.unwrap(),
                    span: *span,
                })
            }

            //    e : Bool
            // ---------------
            //  ensure e : ()
            Statement::Ensure { expression, span } if !self.cfg.allows_ensure_and_require => {
                stmt_not_allowed!("ensure", self, span)
            }
            Statement::Ensure { expression, span } => {
                let mut diags = vec![];
                let mut expr_tc = ExpressionTypeChecker::new(self.source_name, self.ctx);
                let expression = extract_result(expression.accept(&mut expr_tc), &mut diags);
                if let Some(expr) = expression.as_ref() {
                    let bool_type = self.ctx.ts().bool_type();
                    self.ctx.add_constraint(bool_type, expr.r#type());
                    extract_result(
                        self.ctx.unify().map_err(|errs| {
                            errs.into_iter()
                                .flat_map(|err| {
                                    err.into_diags(
                                        self.source_name,
                                        Some(*span),
                                        "on ensure statement",
                                    )
                                })
                                .collect()
                        }),
                        &mut diags,
                    );
                }
                if !diags.is_empty() {
                    return Err(diags);
                }
                Ok(Statement::Ensure {
                    expression: expression.unwrap(),
                    span: *span,
                })
            }
            Statement::Let { name, value, span } => {
                let mut expr_tc = ExpressionTypeChecker::new(self.source_name, self.ctx);
                let value = value.accept(&mut expr_tc)?;
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

            Statement::Return { expression, span } if !self.cfg.allows_return => {
                stmt_not_allowed!("return", self, span)
            }
            Statement::Return { expression, span } => {
                let mut expr_tc = ExpressionTypeChecker::new(self.source_name, self.ctx);
                Ok(Statement::Return {
                    expression: expression.accept(&mut expr_tc)?,
                    span: *span,
                })
            }

            //      e : Felt
            // ------------------
            //  increases e : ()
            Statement::Increases { expression, span } if !self.cfg.allows_invariant_stmts => {
                stmt_not_allowed!("increases", self, span)
            }
            Statement::Increases { expression, span } => {
                let mut diags = vec![];
                let mut expr_tc = ExpressionTypeChecker::new(self.source_name, self.ctx);
                let expr = extract_result(expression.accept(&mut expr_tc), &mut diags);
                if let Some(expr) = expr.as_ref() {
                    let felt_type = self.ctx.ts().felt_type();
                    self.ctx.add_constraint(felt_type, expr.r#type());
                    extract_result(
                        self.ctx.unify().map_err(|errs| {
                            errs.into_iter()
                                .flat_map(|err| {
                                    err.into_diags(
                                        self.source_name,
                                        Some(*span),
                                        "on increases statement",
                                    )
                                })
                                .collect()
                        }),
                        &mut diags,
                    );
                }
                if !diags.is_empty() {
                    return Err(diags);
                }
                Ok(Statement::Increases {
                    expression: expr.unwrap(),
                    span: *span,
                })
            }

            //      e : Felt
            // ------------------
            //  decreases e : ()
            Statement::Decreases { expression, span } if !self.cfg.allows_invariant_stmts => {
                stmt_not_allowed!("decreases", self, span)
            }
            Statement::Decreases { expression, span } => {
                let mut diags = vec![];
                let mut expr_tc = ExpressionTypeChecker::new(self.source_name, self.ctx);
                let expr = extract_result(expression.accept(&mut expr_tc), &mut diags);
                if let Some(expr) = expr.as_ref() {
                    let felt_type = self.ctx.ts().felt_type();
                    self.ctx.add_constraint(felt_type, expr.r#type());
                    extract_result(
                        self.ctx.unify().map_err(|errs| {
                            errs.into_iter()
                                .flat_map(|err| {
                                    err.into_diags(
                                        self.source_name,
                                        Some(*span),
                                        "on decreases statement",
                                    )
                                })
                                .collect()
                        }),
                        &mut diags,
                    );
                }
                if !diags.is_empty() {
                    return Err(diags);
                }
                Ok(Statement::Decreases {
                    expression: expr.unwrap(),
                    span: *span,
                })
            }

            //   e : Bool
            // -------------
            //  step e : ()
            Statement::Step { expression, span } if !self.cfg.allows_invariant_stmts => {
                stmt_not_allowed!("step", self, span)
            }
            Statement::Step { expression, span } => {
                let mut diags = vec![];
                let mut expr_tc = ExpressionTypeChecker::new_with_cfg(
                    self.source_name,
                    self.ctx,
                    ExpressionTypeCheckerCfg { allows_old: true },
                );
                let expr = extract_result(expression.accept(&mut expr_tc), &mut diags);
                if let Some(expr) = expr.as_ref() {
                    let bool_type = self.ctx.ts().bool_type();
                    self.ctx.add_constraint(bool_type, expr.r#type());
                    extract_result(
                        self.ctx.unify().map_err(|errs| {
                            errs.into_iter()
                                .flat_map(|err| {
                                    err.into_diags(
                                        self.source_name,
                                        Some(*span),
                                        "on step statement",
                                    )
                                })
                                .collect()
                        }),
                        &mut diags,
                    );
                }
                if !diags.is_empty() {
                    return Err(diags);
                }
                Ok(Statement::Step {
                    expression: expr.unwrap(),
                    span: *span,
                })
            }

            Statement::Invariant(decl) if !self.cfg.allows_invariants => {
                Err(vec![Diagnostic::new(
                    self.source_name,
                    "invariant declaration not allowed in this context",
                    Some(decl.span()),
                )])
            }
            Statement::Invariant(decl) => {
                let mut inv_tc = InvariantTypeChecker::new(self.ctx, self.source_name);
                decl.accept(&mut inv_tc).map(Statement::Invariant)
            }

            Statement::Predicate(decl) => {
                let mut pred_tc = PredicateTypeChecker::new(self.ctx, self.source_name);
                decl.accept(&mut pred_tc).map(Statement::Predicate)
            }

            Statement::Empty { span } => Ok(Statement::Empty { span: *span }),
        }
    }
}
