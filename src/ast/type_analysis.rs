//! Type analysis of the AST.

use llzk::dialect::bool;
use melior::ir::Type;

use crate::{
    ast::{
        BinaryOp, Block, ContractDecl, Document, Expression, Identifier, Item, PredicateDecl, Span,
        Spanned as _, Statement, UnaryOp, Visitable, Visitor,
    },
    diagnostic::Diagnostic,
    ir::Context as IrContext,
};

pub type TypedDocument<'ast, 'ctx> = Document<'ast, Type<'ctx>>;
pub type TypedItem<'ast, 'ctx> = Item<'ast, Type<'ctx>>;
pub type TypedContractDecl<'ast, 'ctx> = ContractDecl<'ast, Type<'ctx>>;
pub type TypedPredicateDecl<'ast, 'ctx> = PredicateDecl<'ast, Type<'ctx>>;
pub type TypedBlock<'ast, 'ctx> = Block<'ast, Type<'ctx>>;
pub type TypedStatement<'ast, 'ctx> = Statement<'ast, Type<'ctx>>;
pub type TypedExpression<'ast, 'ctx> = Expression<'ast, Type<'ctx>>;

type TypingResult<T> = Result<T, Vec<Diagnostic>>;

struct TypeChecker<'ctx, 'name> {
    ctx: &'ctx IrContext,
    source_name: &'name str,
}

fn check_many<'a, V, I, O, E, R>(
    visitor: &mut V,
    entities: impl IntoIterator<Item = &'a I>,
    combine: impl FnOnce(Vec<O>) -> R,
) -> Result<R, Vec<E>>
where
    I: Visitable + 'a,
    V: Visitor<I, Output = Result<O, Vec<E>>>,
{
    let mut errs = vec![];
    let mut results = vec![];
    for entity in entities {
        match entity.accept(visitor) {
            Ok(result) => results.push(result),
            Err(err) => errs.extend(err),
        }
    }
    if errs.is_empty() {
        Ok(combine(results))
    } else {
        Err(errs)
    }
}

impl<'ast, 'ctx, 'name> Visitor<Document<'ast>> for TypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedDocument<'ast, 'ctx>>;

    fn visit(&mut self, document: &Document<'ast>) -> Self::Output {
        check_many(self, &document.items, |items| Document {
            items,
            span: document.span,
        })
    }
}

impl<'ast, 'ctx, 'name> Visitor<Item<'ast>> for TypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedItem<'ast, 'ctx>>;

    fn visit(&mut self, entity: &Item<'ast>) -> Self::Output {
        match entity {
            Item::Contract(decl) => decl
                .accept(&mut ContractTypeChecker::new(self.ctx, self.source_name))
                .map(Into::into),
            Item::Predicate(decl) => decl
                .accept(&mut PredicateTypeChecker::new(self.ctx, self.source_name))
                .map(Into::into),
        }
    }
}

struct ContractTypeChecker<'ctx, 'name> {
    ctx: &'ctx IrContext,
    source_name: &'name str,
}

impl<'ctx, 'name> ContractTypeChecker<'ctx, 'name> {
    fn new(ctx: &'ctx IrContext, source_name: &'name str) -> Self {
        Self { ctx, source_name }
    }
}

impl<'ast, 'ctx, 'name> Visitor<ContractDecl<'ast>> for ContractTypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedContractDecl<'ast, 'ctx>>;

    fn visit(&mut self, _: &ContractDecl<'ast>) -> Self::Output {
        todo!("contract type checking is not implemented yet")
    }
}

struct PredicateTypeChecker<'ctx, 'name> {
    ctx: &'ctx IrContext,
    source_name: &'name str,
}

impl<'ctx, 'name> PredicateTypeChecker<'ctx, 'name> {
    fn new(ctx: &'ctx IrContext, source_name: &'name str) -> Self {
        Self { ctx, source_name }
    }

    fn ensure_no_early_return(&self, block: &TypedBlock<'_, 'ctx>) -> Vec<Diagnostic> {
        block
            .statements
            .iter()
            .rev()
            // Skip the last statement since that one is allowed to be a return.
            .skip(1)
            .rev()
            .filter_map(|stmt| {
                matches!(stmt, Statement::Return { .. }).then(|| {
                    Diagnostic::new(
                        self.source_name,
                        format!("return statements must be the last statement in a predicate"),
                        Some(stmt.span()),
                    )
                })
            })
            .collect()
    }

    fn ensure_return_terminator(&self, block: &TypedBlock<'_, 'ctx>) -> Option<Diagnostic> {
        let Some(last) = block.statements.last() else {
            return Some(Diagnostic::new(
                self.source_name,
                format!("predicates cannot have an empty body"),
                Some(block.span),
            ));
        };

        match last {
            Statement::Return { expression, span } => {
                let bool_type = self.ctx.bool_type();
                (!expression.has_type(bool_type)).then(|| {
                    Diagnostic::new(
                        self.source_name,
                        format!(
                            "predicates must return a boolean expression. Got '{}'",
                            expression.r#type()
                        ),
                        Some(*span),
                    )
                })
            }
            _ => Some(Diagnostic::new(
                self.source_name,
                format!("predicates with a block body must end with a return statement"),
                Some(block.span),
            )),
        }
    }
}

impl<'ast, 'ctx, 'name> Visitor<PredicateDecl<'ast>> for PredicateTypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedPredicateDecl<'ast, 'ctx>>;

    fn visit(&mut self, decl: &PredicateDecl<'ast>) -> Self::Output {
        let mut block_tc = BlockTypeChecker {
            ctx: self.ctx,
            source_name: self.source_name,
            allows_invariants: false,
            allows_scoped: false,
        };
        let mut diags = vec![];
        let body = decl.body.accept(&mut block_tc)?;

        // Check that the last statement is a return and no other statement is.
        diags.extend(self.ensure_no_early_return(&body));
        diags.extend(self.ensure_return_terminator(&body));

        if !diags.is_empty() {
            return Err(diags);
        }
        // TODO: Put the deduced type of the params on these identifiers
        let ins = vec![self.ctx.bool_type(); decl.params.len()];
        let params = std::iter::zip(&decl.params, &ins)
            .map(|(ident, r#type)| Identifier {
                name: ident.name,
                span: ident.span,
                meta: *r#type,
            })
            .collect::<Vec<_>>();
        Ok(PredicateDecl {
            name: Identifier {
                name: decl.name.name,
                span: decl.name.span,
                meta: self.ctx.func_type(&ins, &[self.ctx.bool_type()]).into(),
            },
            params,
            body,
            span: decl.span,
        })
    }
}

struct BlockTypeChecker<'ctx, 'name> {
    ctx: &'ctx IrContext,
    source_name: &'name str,
    /// Wether invariant declarations are allowed in this context.
    allows_invariants: bool,
    /// Wether scoped blocks are allowed in this context.
    allows_scoped: bool,
}

impl<'ctx> BlockTypeChecker<'ctx, '_> {
    fn ensure_type(
        &self,
        expected: Type<'ctx>,
        actual: Type<'ctx>,
        span: Span,
        header: &str,
    ) -> Option<Diagnostic> {
        (expected != actual).then(|| {
            Diagnostic::new(
                self.source_name,
                format!("type mismatch on {header}: expected type {expected} but got {actual}"),
                Some(span),
            )
        })
    }

    fn check_binary_op_types(
        &self,
        op: BinaryOp,
        left: Option<&TypedExpression<'_, 'ctx>>,
        right: Option<&TypedExpression<'_, 'ctx>>,
        diags: &mut Vec<Diagnostic>,
    ) {
        let expected = op.expected_type(self.ctx);

        for expr in [left, right] {
            if let Some(expr) = expr {
                diags.extend(self.ensure_type(
                    expected,
                    expr.r#type(),
                    expr.span(),
                    "on expression",
                ));
            }
        }
    }

    fn check_unary_op_types(
        &self,
        op: UnaryOp,
        expr: Option<&TypedExpression<'_, 'ctx>>,
        diags: &mut Vec<Diagnostic>,
    ) {
        if let Some(expr) = expr {
            diags.extend(self.ensure_type(
                op.expected_type(self.ctx),
                expr.r#type(),
                expr.span(),
                "on expression",
            ));
        }
    }
}

impl<'ast, 'ctx, 'name> Visitor<Block<'ast>> for BlockTypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedBlock<'ast, 'ctx>>;

    fn visit(&mut self, block: &Block<'ast>) -> Self::Output {
        check_many(self, &block.statements, |statements| Block {
            statements,
            span: block.span,
        })
    }
}

impl<'ast, 'ctx, 'name> Visitor<Statement<'ast>> for BlockTypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedStatement<'ast, 'ctx>>;

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
            Statement::Block(block) => block.accept(self).map(Statement::Block),
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
                let name = Identifier {
                    name: name.name,
                    span: name.span,
                    meta: value.r#type(),
                };
                Ok(Statement::Let {
                    name,
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
                        Some(decl.span),
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

fn extract_result<T>(r: TypingResult<T>, diags: &mut Vec<Diagnostic>) -> Option<T> {
    match r {
        Ok(r) => Some(r),
        Err(e) => {
            diags.extend(e);
            None
        }
    }
}

impl<'ast, 'ctx, 'name> Visitor<Expression<'ast>> for BlockTypeChecker<'ctx, 'name> {
    type Output = TypingResult<TypedExpression<'ast, 'ctx>>;

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

                let bool_type = self.ctx.bool_type();
                if let Some(condition) = &condition {
                    diags.extend(self.ensure_type(
                        bool_type,
                        condition.r#type(),
                        condition.span(),
                        "conditional expression",
                    ));
                }

                if let Some((then_branch, else_branch)) =
                    then_branch.as_ref().zip(else_branch.as_ref())
                {
                    diags.extend(self.ensure_type(
                        then_branch.r#type(),
                        else_branch.r#type(),
                        *span,
                        "conditional expression branches",
                    ));
                }

                if !diags.is_empty() {
                    return Err(diags);
                }

                let then_branch = then_branch.unwrap();
                let return_type = then_branch.r#type();

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
                self.check_binary_op_types(*op, left.as_ref(), right.as_ref(), &mut diags);
                if !diags.is_empty() {
                    return Err(diags);
                }

                Ok(Expression::Binary {
                    op: *op,
                    left: Box::new(left.unwrap()),
                    right: Box::new(right.unwrap()),
                    span: *span,
                    meta: op.return_type(self.ctx),
                })
            }
            Expression::Unary { op, expr, span, .. } => {
                let mut diags = vec![];
                let expr = extract_result(expr.accept(self), &mut diags);
                self.check_unary_op_types(*op, expr.as_ref(), &mut diags);
                if !diags.is_empty() {
                    return Err(diags);
                }

                Ok(Expression::Unary {
                    op: *op,
                    expr: Box::new(expr.unwrap()),
                    span: *span,
                    meta: op.return_type(self.ctx),
                })
            }
            Expression::Index { .. } => todo!(),
            Expression::Member { .. } => todo!(),
            Expression::Call { .. } => todo!(),
            Expression::Quantifier { .. } => todo!(),
            Expression::Len { .. } => todo!(),
            Expression::Old { .. } => todo!(),
            Expression::Arg { .. } => todo!(),
            Expression::Nondet { span, .. } => Ok(Expression::Nondet {
                span: *span,
                meta: self.ctx.felt_type(),
            }),
            Expression::Boolean { value, span, .. } => Ok(Expression::Boolean {
                value: *value,
                span: *span,
                meta: self.ctx.bool_type(),
            }),
            Expression::Number { value, span, .. } => Ok(Expression::Number {
                value: *value,
                span: *span,
                meta: self.ctx.felt_type(),
            }),
            Expression::Symbol(_) => todo!(),
        }
    }
}

impl<'ast, 'ctx> TypedExpression<'ast, 'ctx> {
    /// Returns the type of the expression.
    pub fn r#type(&self) -> Type<'ctx> {
        *self.meta()
    }

    /// Returns true if the expression is of the given type.
    pub fn has_type(&self, r#type: Type<'ctx>) -> bool {
        self.r#type() == r#type
    }
}

impl BinaryOp {
    fn expected_type<'ctx>(&self, ctx: &'ctx IrContext) -> Type<'ctx> {
        match self {
            BinaryOp::Or | BinaryOp::And => ctx.bool_type(),
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::BitAnd
            | BinaryOp::Pow => ctx.felt_type(),
        }
    }

    fn return_type<'ctx>(&self, ctx: &'ctx IrContext) -> Type<'ctx> {
        match self {
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => ctx.bool_type(),
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::BitAnd
            | BinaryOp::Pow => ctx.felt_type(),
        }
    }
}

impl UnaryOp {
    fn expected_type<'ctx>(&self, ctx: &'ctx IrContext) -> Type<'ctx> {
        match self {
            UnaryOp::Not => ctx.bool_type(),
            UnaryOp::Neg => ctx.felt_type(),
        }
    }

    fn return_type<'ctx>(&self, ctx: &'ctx IrContext) -> Type<'ctx> {
        match self {
            UnaryOp::Not => ctx.bool_type(),
            UnaryOp::Neg => ctx.felt_type(),
        }
    }
}
