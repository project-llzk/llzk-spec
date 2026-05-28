use std::ops::{Deref, DerefMut};

use crate::{
    ast::{Block, Spanned as _, Statement, Visitable as _, Visitor},
    type_analysis::{
        TypeSystem, TypingResult,
        base::BaseTypeChecker,
        ctx::TypeInferenceCtx,
        expression::{ExpressionTypeChecker, ExpressionTypeCheckerCfg},
        helpers::{Diagnostics, check_many},
        invariant::InvariantTypeChecker,
        predicate::PredicateTypeChecker,
    },
};

/// Configuration for type-checking a block.
///
/// Different declaration-like entities have different rules regarding what goes into a block. This
/// configuration allows reusing the same type-checker across these declarations.
pub(super) struct BlockTypeCheckerCfg {
    /// Whether invariant declarations are allowed in this context.
    ///
    /// Note that this configuration parameter states if the `invariant` declaration statement is
    /// allowed, not statements allowed inside an invariant declaration. Use `allows_invariant_stmts` for that purpose.
    pub allows_invariants: bool,
    /// Whether scoped blocks are allowed in this context.
    pub allows_scoped: bool,
    /// Whether ensure and require statements are allowed in this context.
    pub allows_ensure_and_require: bool,
    /// Whether return statements are allowed in this context.
    pub allows_return: bool,
    /// Whether increases, decreases, and step statements are allowed in this context.
    ///
    /// This parameter does not state that the `invariant` declaration statement is allowed. Use
    /// `allows_invariants` for that.
    pub allows_invariant_stmts: bool,
    /// Whether the `arg` expression is allowed.
    pub allows_arg: bool,
}

/// Configurable type checker of generic blocks of code.
///
/// Declaration-like AST entities can reuse this type checker by passing the correct configuration
/// for their particular semantics.
pub(super) struct BlockTypeChecker<'ctx, 'ast, T: TypeSystem> {
    base: BaseTypeChecker<'ctx, 'ast, T>,
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
            base: BaseTypeChecker::new(ctx, source_name),
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

/// Helper for creating an `$stmt statement is not allowed in this context` error message.
macro_rules! stmt_not_allowed {
    ($name:literal) => {
        concat!($name, " statement is not allowed in this context")
    };
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Statement<'ast>> for BlockTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Statement<'ast, T::Type>>;

    fn visit(&mut self, statement: &Statement<'ast>) -> Self::Output {
        let mut diags = Diagnostics::new(self.source_name, statement);
        let allows_arg = self.cfg.allows_arg;
        match statement {
            Statement::Scoped {
                scope,
                statement,
                span,
            } => {
                let mut inner = None;
                if diags.add_unless(self.cfg.allows_scoped, || stmt_not_allowed!("scoped block")) {
                    self.ctx.scope().push(());
                    inner = diags.extract_result(statement.accept(self));
                    self.ctx.scope().pop();
                }
                diags.finish(|| Statement::Scoped {
                    scope: *scope,
                    statement: Box::new(inner.unwrap()),
                    span: *span,
                })
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
            Statement::Require { expression, span } => {
                let expression = if diags.add_unless(self.cfg.allows_ensure_and_require, || {
                    stmt_not_allowed!("require")
                }) {
                    let mut expr_tc =
                        ExpressionTypeChecker::new(self.source_name, self.ctx, allows_arg);
                    let expression = diags.extract_result(expression.accept(&mut expr_tc));
                    self.constrain_to_bool(expression.as_ref());
                    self.unify(span, || "on require statement", &mut diags);
                    expression
                } else {
                    None
                };
                diags.finish(|| Statement::Require {
                    expression: expression.unwrap(),
                    span: *span,
                })
            }

            //    e : Bool
            // ---------------
            //  ensure e : ()
            Statement::Ensure { expression, span } => {
                let expression = if diags.add_unless(self.cfg.allows_ensure_and_require, || {
                    stmt_not_allowed!("ensure")
                }) {
                    let mut expr_tc =
                        ExpressionTypeChecker::new(self.source_name, self.ctx, allows_arg);
                    let expression = diags.extract_result(expression.accept(&mut expr_tc));
                    self.constrain_to_bool(expression.as_ref());
                    self.unify(span, || "on ensure statement", &mut diags);
                    expression
                } else {
                    None
                };
                diags.finish(|| Statement::Ensure {
                    expression: expression.unwrap(),
                    span: *span,
                })
            }
            Statement::Let { name, value, span } => {
                let mut expr_tc =
                    ExpressionTypeChecker::new(self.source_name, self.ctx, allows_arg);
                let value = diags.extract_result(value.accept(&mut expr_tc));
                if let Some(value) = value.as_ref() {
                    diags.extract_type_result(
                        self.ctx.scope().top().bind_local(name, value.r#type()),
                        || "in let statement",
                    );
                }

                diags.finish(|| {
                    let value = value.unwrap();
                    Statement::Let {
                        name: name.with_meta(value.r#type()),
                        value,
                        span: *span,
                    }
                })
            }

            Statement::Unused { name, span } => diags
                .to_typing_result(self.ctx.scope().find_local(name).cloned(), || {
                    format!("on symbol '{}'", name.value())
                })
                .map(|t| Statement::Unused {
                    name: name.with_meta(t),
                    span: *span,
                }),

            Statement::Return { expression, span } => {
                let expression =
                    if diags.add_unless(self.cfg.allows_return, || stmt_not_allowed!("return")) {
                        let mut expr_tc =
                            ExpressionTypeChecker::new(self.source_name, self.ctx, allows_arg);
                        diags.extract_result(expression.accept(&mut expr_tc))
                    } else {
                        None
                    };

                diags.finish(|| Statement::Return {
                    expression: expression.unwrap(),
                    span: *span,
                })
            }

            //      e : Felt
            // ------------------
            //  increases e : ()
            Statement::Increases { expression, span } => {
                let mut expr = None;
                if diags.add_unless(self.cfg.allows_invariant_stmts, || {
                    stmt_not_allowed!("increases")
                }) {
                    let mut expr_tc =
                        ExpressionTypeChecker::new(self.source_name, self.ctx, allows_arg);
                    expr = diags.extract_result(expression.accept(&mut expr_tc));
                    self.constrain_to_felt(expr.as_ref());
                    self.unify(span, || "on increases statement", &mut diags);
                }
                diags.finish(|| Statement::Increases {
                    expression: expr.unwrap(),
                    span: *span,
                })
            }

            //      e : Felt
            // ------------------
            //  decreases e : ()
            Statement::Decreases { expression, span } => {
                let mut expr = None;
                if diags.add_unless(self.cfg.allows_invariant_stmts, || {
                    stmt_not_allowed!("decreases")
                }) {
                    let mut expr_tc =
                        ExpressionTypeChecker::new(self.source_name, self.ctx, allows_arg);
                    expr = diags.extract_result(expression.accept(&mut expr_tc));
                    self.constrain_to_felt(expr.as_ref());
                    self.unify(span, || "on decreases statement", &mut diags);
                }
                diags.finish(|| Statement::Decreases {
                    expression: expr.unwrap(),
                    span: *span,
                })
            }

            //   e : Bool
            // -------------
            //  step e : ()
            Statement::Step { expression, span } => {
                let mut expr = None;
                if diags.add_unless(self.cfg.allows_invariant_stmts, || {
                    stmt_not_allowed!("step")
                }) {
                    let mut expr_tc = ExpressionTypeChecker::new_with_cfg(
                        self.source_name,
                        self.ctx,
                        ExpressionTypeCheckerCfg {
                            allows_old: true,
                            allows_arg,
                        },
                    );
                    expr = diags.extract_result(expression.accept(&mut expr_tc));
                    self.constrain_to_bool(expr.as_ref());
                    self.unify(span, || "on step statement", &mut diags);
                }
                diags.finish(|| Statement::Step {
                    expression: expr.unwrap(),
                    span: *span,
                })
            }

            Statement::Invariant(decl) => {
                let decl = if diags.add_unless(self.cfg.allows_invariants, || {
                    stmt_not_allowed!("invariant declaration")
                }) {
                    let mut inv_tc = InvariantTypeChecker::new(self.source_name, self.ctx);
                    diags.extract_result(decl.accept(&mut inv_tc))
                } else {
                    None
                };
                diags.finish(|| Statement::Invariant(decl.unwrap()))
            }

            Statement::Predicate(decl) => {
                let mut pred_tc = PredicateTypeChecker::new(self.source_name, self.ctx);
                let decl = diags.extract_result(decl.accept(&mut pred_tc));
                diags.finish(|| Statement::Predicate(decl.unwrap()))
            }

            Statement::Empty { span } => diags.finish(|| Statement::Empty { span: *span }),
        }
    }
}

impl<'ctx, 'ast, T: TypeSystem> DerefMut for BlockTypeChecker<'ctx, 'ast, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<'ctx, 'ast, T: TypeSystem> Deref for BlockTypeChecker<'ctx, 'ast, T> {
    type Target = BaseTypeChecker<'ctx, 'ast, T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
