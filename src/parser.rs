use crate::ast::*;
use crate::diagnostic::Diagnostic;
use pest::Parser;
use pest::Span as PestSpan;
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "../llzk-spec.pest"]
struct LlzkSpecParser;

pub fn parse_document(source_name: &str, source: &str) -> Result<Document, Diagnostic> {
    let pairs = LlzkSpecParser::parse(Rule::file, source).map_err(|error| {
        let (line, column) = match error.line_col {
            pest::error::LineColLocation::Pos((line, column)) => (line, column),
            pest::error::LineColLocation::Span((line, column), _) => (line, column),
        };
        Diagnostic::new(
            source_name,
            format!("syntax error: {error}"),
            Some(Span {
                start: 0,
                end: 0,
                line,
                column,
            }),
        )
    })?;

    Lowerer { source_name }.document(pairs)
}

struct Lowerer<'a> {
    source_name: &'a str,
}

impl<'a> Lowerer<'a> {
    fn document(&self, mut pairs: Pairs<'a, Rule>) -> Result<Document, Diagnostic> {
        let file = pairs.next().expect("file pair");
        let items = file
            .into_inner()
            .filter(|pair| pair.as_rule() != Rule::EOI)
            .map(|pair| self.item(pair))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Document { items })
    }

    fn item(&self, pair: Pair<'a, Rule>) -> Result<Item, Diagnostic> {
        match pair.as_rule() {
            Rule::contract_decl => Ok(Item::Contract(self.contract_decl(pair)?)),
            Rule::predicate_decl => Ok(Item::Predicate(self.predicate_decl(pair)?)),
            _ => self.unexpected(pair, "item"),
        }
    }

    fn contract_decl(&self, pair: Pair<'a, Rule>) -> Result<ContractDecl, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let target = self.identifier(inner.next().expect("contract target"))?;
        let body = self.block(inner.next().expect("contract body"))?;
        Ok(ContractDecl { target, body, span })
    }

    fn predicate_decl(&self, pair: Pair<'a, Rule>) -> Result<PredicateDecl, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let name = self.identifier(inner.next().expect("predicate name"))?;
        let params = self.param_list(inner.next().expect("predicate params"))?;
        let body_pair = inner.next().expect("predicate body");
        let body = match body_pair.as_rule() {
            Rule::block_predicate_body => PredicateBody::Block(
                self.block(body_pair.into_inner().next().expect("predicate block"))?,
            ),
            _ => PredicateBody::Expr(self.expression(body_pair)?),
        };

        Ok(PredicateDecl {
            name,
            params,
            body,
            span,
        })
    }

    fn param_list(&self, pair: Pair<'a, Rule>) -> Result<Vec<Identifier>, Diagnostic> {
        pair.into_inner()
            .map(|pair| self.identifier(pair))
            .collect()
    }

    fn block(&self, pair: Pair<'a, Rule>) -> Result<Block, Diagnostic> {
        let span = self.span(pair.as_span());
        let statements = pair
            .into_inner()
            .map(|pair| self.statement(pair))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Block { statements, span })
    }

    fn statement(&self, pair: Pair<'a, Rule>) -> Result<Statement, Diagnostic> {
        match pair.as_rule() {
            Rule::statement => self.statement(pair.into_inner().next().expect("statement")),
            Rule::scoped_stmt => self.scoped_stmt(pair),
            Rule::require_stmt => {
                let span = self.span(pair.as_span());
                let expression =
                    self.expression(pair.into_inner().next().expect("require expr"))?;
                Ok(Statement::Require { expression, span })
            }
            Rule::ensure_stmt => {
                let span = self.span(pair.as_span());
                let expression = self.expression(pair.into_inner().next().expect("ensure expr"))?;
                Ok(Statement::Ensure { expression, span })
            }
            Rule::let_stmt => {
                let span = self.span(pair.as_span());
                let mut inner = pair.into_inner();
                let name = self.identifier(inner.next().expect("let name"))?;
                let value = self.expression(inner.next().expect("let value"))?;
                Ok(Statement::Let { name, value, span })
            }
            Rule::unused_stmt => {
                let span = self.span(pair.as_span());
                let name = self.identifier(pair.into_inner().next().expect("unused name"))?;
                Ok(Statement::Unused { name, span })
            }
            Rule::return_stmt => {
                let span = self.span(pair.as_span());
                let expression = self.expression(pair.into_inner().next().expect("return expr"))?;
                Ok(Statement::Return { expression, span })
            }
            Rule::invariant_decl => Ok(Statement::Invariant(self.invariant_decl(pair)?)),
            Rule::predicate_decl => Ok(Statement::Predicate(self.predicate_decl(pair)?)),
            Rule::semi => Ok(Statement::Empty {
                span: self.span(pair.as_span()),
            }),
            Rule::block => Ok(Statement::Block(self.block(pair)?)),
            _ => self.unexpected(pair, "statement"),
        }
    }

    fn scoped_stmt(&self, pair: Pair<'a, Rule>) -> Result<Statement, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let scope = match inner.next().expect("scope prefix").as_str() {
            "compute" => Scope::Compute,
            "witness" => Scope::Witness,
            "constrain" => Scope::Constrain,
            other => {
                return Err(Diagnostic::new(
                    self.source_name,
                    format!("unknown scope prefix `{other}`"),
                    Some(span),
                ));
            }
        };
        let statement = Box::new(self.statement(inner.next().expect("scoped statement"))?);
        Ok(Statement::Scoped {
            scope,
            statement,
            span,
        })
    }

    fn invariant_decl(&self, pair: Pair<'a, Rule>) -> Result<InvariantDecl, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let loop_label = self.identifier(inner.next().expect("loop label"))?;
        let induction_var = self.identifier(inner.next().expect("induction variable"))?;
        let body = self.block(inner.next().expect("invariant body"))?;
        Ok(InvariantDecl {
            loop_label,
            induction_var,
            body,
            span,
        })
    }

    fn expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        match pair.as_rule() {
            Rule::expression => self.expression(pair.into_inner().next().expect("expression")),
            Rule::conditional_expr => self.conditional_expression(pair),
            Rule::logical_or_expr
            | Rule::logical_and_expr
            | Rule::equality_expr
            | Rule::relational_expr
            | Rule::additive_expr
            | Rule::multiplicative_expr => self.binary_expression(pair),
            Rule::power_expr => self.power_expression(pair),
            Rule::unary_expr => self.unary_expression(pair),
            Rule::postfix_expr => self.postfix_expression(pair),
            Rule::primary_expr => self.expression(pair.into_inner().next().expect("primary expr")),
            Rule::quantifier_expr => self.quantifier_expression(pair),
            Rule::len_expr => {
                let span = self.span(pair.as_span());
                let target = self.expression(pair.into_inner().next().expect("len target"))?;
                Ok(Expression::Len {
                    target: Box::new(target),
                    span,
                })
            }
            Rule::nondet_expr => Ok(Expression::Nondet {
                span: self.span(pair.as_span()),
            }),
            Rule::boolean => Ok(Expression::Boolean {
                value: pair.as_str() == "true",
                span: self.span(pair.as_span()),
            }),
            Rule::number => Ok(Expression::Number {
                value: pair.as_str().to_string(),
                span: self.span(pair.as_span()),
            }),
            Rule::symbol => Ok(Expression::Symbol(self.identifier(pair)?)),
            _ => self.unexpected(pair, "expression"),
        }
    }

    fn conditional_expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let condition = self.expression(inner.next().expect("condition"))?;
        if let Some(then_pair) = inner.next() {
            let else_pair = inner.next().expect("else branch");
            Ok(Expression::Conditional {
                condition: Box::new(condition),
                then_branch: Box::new(self.expression(then_pair)?),
                else_branch: Box::new(self.expression(else_pair)?),
                span,
            })
        } else {
            Ok(condition)
        }
    }

    fn binary_expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let mut expr = self.expression(inner.next().expect("binary lhs"))?;
        while let Some(op_pair) = inner.next() {
            let right = self.expression(inner.next().expect("binary rhs"))?;
            expr = Expression::Binary {
                op: self.binary_op(op_pair.as_rule(), op_pair.as_str())?,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn power_expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let left = self.expression(inner.next().expect("power lhs"))?;
        if let Some(op_pair) = inner.next() {
            let right = self.expression(inner.next().expect("power rhs"))?;
            Ok(Expression::Binary {
                op: self.binary_op(op_pair.as_rule(), op_pair.as_str())?,
                left: Box::new(left),
                right: Box::new(right),
                span,
            })
        } else {
            Ok(left)
        }
    }

    fn unary_expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner().peekable();
        let mut ops = Vec::new();
        while inner
            .peek()
            .is_some_and(|pair| pair.as_rule() == Rule::unary_op)
        {
            let op = match inner.next().expect("unary op").as_str() {
                "!" => UnaryOp::Not,
                "-" => UnaryOp::Neg,
                other => {
                    return Err(Diagnostic::new(
                        self.source_name,
                        format!("unknown unary operator `{other}`"),
                        Some(span),
                    ));
                }
            };
            ops.push(op);
        }

        let mut expr = self.expression(inner.next().expect("unary target"))?;
        for op in ops.into_iter().rev() {
            expr = Expression::Unary {
                op,
                expr: Box::new(expr),
                span,
            };
        }
        Ok(expr)
    }

    fn postfix_expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let mut expr = self.expression(inner.next().expect("postfix base"))?;
        for suffix in inner {
            match suffix.as_rule() {
                Rule::index_op => {
                    let index = self.expression(suffix.into_inner().next().expect("index expr"))?;
                    expr = Expression::Index {
                        target: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                Rule::call_suffix => {
                    let args = suffix
                        .into_inner()
                        .map(|pair| self.expression(pair))
                        .collect::<Result<Vec<_>, _>>()?;
                    let callee = match expr {
                        Expression::Symbol(identifier) => identifier,
                        other => {
                            return Err(Diagnostic::new(
                                self.source_name,
                                "only named predicate calls are supported in phase 1",
                                Some(other.span()),
                            ));
                        }
                    };
                    expr = Expression::Call { callee, args, span };
                }
                _ => return self.unexpected(suffix, "postfix suffix"),
            }
        }
        Ok(expr)
    }

    fn quantifier_expression(&self, pair: Pair<'a, Rule>) -> Result<Expression, Diagnostic> {
        let span = self.span(pair.as_span());
        let mut inner = pair.into_inner();
        let kind = match inner.next().expect("quantifier kind").as_str() {
            "forall" => QuantifierKind::Forall,
            "exists" => QuantifierKind::Exists,
            other => {
                return Err(Diagnostic::new(
                    self.source_name,
                    format!("unknown quantifier `{other}`"),
                    Some(span),
                ));
            }
        };
        let binding = self.identifier(inner.next().expect("quantifier binding"))?;
        let domain = self.quantifier_domain(inner.next().expect("quantifier domain"))?;
        let body = self.expression(inner.next().expect("quantifier body"))?;
        Ok(Expression::Quantifier {
            quantifier_kind: kind,
            binding,
            domain,
            body: Box::new(body),
            span,
        })
    }

    fn quantifier_domain(&self, pair: Pair<'a, Rule>) -> Result<QuantifierDomain, Diagnostic> {
        let pair = if pair.as_rule() == Rule::quantifier_domain {
            pair.into_inner().next().expect("quantifier domain body")
        } else {
            pair
        };

        match pair.as_rule() {
            Rule::range_expr => {
                let span = self.span(pair.as_span());
                let mut inner = pair.into_inner();
                let start = self.expression(inner.next().expect("range start"))?;
                let end = self.expression(inner.next().expect("range end"))?;
                Ok(QuantifierDomain::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                    span,
                })
            }
            _ => Ok(QuantifierDomain::Expr(Box::new(self.expression(pair)?))),
        }
    }

    fn identifier(&self, pair: Pair<'a, Rule>) -> Result<Identifier, Diagnostic> {
        match pair.as_rule() {
            Rule::symbol => Ok(Identifier {
                name: pair.as_str().to_string(),
                span: self.span(pair.as_span()),
            }),
            _ => self.unexpected(pair, "identifier"),
        }
    }

    fn binary_op(&self, rule: Rule, text: &str) -> Result<BinaryOp, Diagnostic> {
        let op = match (rule, text) {
            (Rule::or_op, "||") => BinaryOp::Or,
            (Rule::and_op, "&&") => BinaryOp::And,
            (Rule::eq_op, "==") => BinaryOp::Eq,
            (Rule::eq_op, "!=") => BinaryOp::Ne,
            (Rule::rel_op, "<") => BinaryOp::Lt,
            (Rule::rel_op, "<=") => BinaryOp::Le,
            (Rule::rel_op, ">") => BinaryOp::Gt,
            (Rule::rel_op, ">=") => BinaryOp::Ge,
            (Rule::add_op, "+") => BinaryOp::Add,
            (Rule::add_op, "-") => BinaryOp::Sub,
            (Rule::mul_op, "*") => BinaryOp::Mul,
            (Rule::mul_op, "/") => BinaryOp::Div,
            (Rule::mul_op, "%") => BinaryOp::Mod,
            (Rule::mul_op, "&") => BinaryOp::BitAnd,
            (Rule::pow_op, "**") => BinaryOp::Pow,
            _ => {
                return Err(Diagnostic::new(
                    self.source_name,
                    format!("unknown operator `{text}`"),
                    None,
                ));
            }
        };
        Ok(op)
    }

    fn span(&self, span: PestSpan<'a>) -> Span {
        let (line, column) = span.start_pos().line_col();
        Span {
            start: span.start(),
            end: span.end(),
            line,
            column,
        }
    }

    fn unexpected<T>(&self, pair: Pair<'a, Rule>, expected: &str) -> Result<T, Diagnostic> {
        Err(Diagnostic::new(
            self.source_name,
            format!("expected {expected}, found {:?}", pair.as_rule()),
            Some(self.span(pair.as_span())),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, Expression, Item, QuantifierDomain, Statement};

    #[test]
    fn parses_contract_and_predicate_forms() {
        let source = r#"
contract for Foo {
  ensure out == 1;
  predicate local(x) = x + 1
}
predicate ok(x) { return x == 0; }
"#;
        let document = parse_document("test.spec", source).expect("parse success");
        assert_eq!(document.items.len(), 2);
        match &document.items[0] {
            Item::Contract(contract) => {
                assert_eq!(contract.target.name, "Foo");
                assert!(matches!(
                    contract.body.statements[1],
                    Statement::Predicate(_)
                ));
            }
            other => panic!("unexpected item: {other:?}"),
        }
    }

    #[test]
    fn preserves_expression_precedence() {
        let source = "predicate p(x) = x + 2 * 3 ** 4";
        let document = parse_document("test.spec", source).expect("parse success");
        let Item::Predicate(predicate) = &document.items[0] else {
            panic!("expected predicate");
        };
        let PredicateBody::Expr(Expression::Binary { op, right, .. }) = &predicate.body else {
            panic!("expected additive expression");
        };
        assert_eq!(*op, BinaryOp::Add);
        let Expression::Binary { op, .. } = right.as_ref() else {
            panic!("expected multiplicative expression");
        };
        assert_eq!(*op, BinaryOp::Mul);
    }

    #[test]
    fn parses_quantifier_domain_range() {
        let source = "predicate p(xs) = forall i in 0..len(xs), xs[i] == 0";
        let document = parse_document("test.spec", source).expect("parse success");
        let Item::Predicate(predicate) = &document.items[0] else {
            panic!("expected predicate");
        };
        let PredicateBody::Expr(Expression::Quantifier { domain, .. }) = &predicate.body else {
            panic!("expected quantifier");
        };
        assert!(matches!(domain, QuantifierDomain::Range { .. }));
    }
}
