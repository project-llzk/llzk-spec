//! Abstract syntax tree for `llzk-spec`.

use num_bigint::BigUint;
use serde::Serialize;

/// Trait defining a visitor of AST entities.
pub trait Visitor<E: Visitable> {
    type Output;

    fn visit(&mut self, entity: &E) -> Self::Output;
}

/// Trait defining a visitable entity of the AST.
pub trait Visitable: Sized {
    fn accept<V>(&self, visitor: &mut V) -> V::Output
    where
        V: Visitor<Self>,
    {
        visitor.visit(self)
    }
}

/// Source location of an AST node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// Identifier with attached source span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Parsed top-level document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Document {
    pub items: Vec<Item>,
    pub span: Span,
}

impl Visitable for Document {}

/// Top-level declarations accepted by the language.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    Contract(ContractDecl),
    Predicate(PredicateDecl),
}

impl Visitable for Item {}

/// Contract declaration attached to an LLZK symbol.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDecl {
    pub target: Identifier,
    pub body: Block,
    pub span: Span,
}

impl Visitable for ContractDecl {}

/// Predicate declaration in block or inline-expression form.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredicateDecl {
    pub name: Identifier,
    pub params: Vec<Identifier>,
    pub body: Block,
    pub span: Span,
}

impl Visitable for PredicateDecl {}

/// Sequence of statements with a lexical scope.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

impl Visitable for Block {}

/// Statements supported by phase 1 of `llzk-spec`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Statement {
    Scoped {
        scope: Scope,
        statement: Box<Statement>,
        span: Span,
    },
    Block(Block),
    Require {
        expression: Expression,
        span: Span,
    },
    Ensure {
        expression: Expression,
        span: Span,
    },
    Let {
        name: Identifier,
        value: Expression,
        span: Span,
    },
    Unused {
        name: Identifier,
        span: Span,
    },
    Return {
        expression: Expression,
        span: Span,
    },
    Increases {
        expression: Expression,
        span: Span,
    },
    Decreases {
        expression: Expression,
        span: Span,
    },
    Step {
        expression: Expression,
        span: Span,
    },
    Invariant(InvariantDecl),
    Predicate(PredicateDecl),
    Empty {
        span: Span,
    },
}

impl Visitable for Statement {}

/// Execution scope qualifier for a statement or block.
///
/// The source keywords `compute` and `witness` both lower to `Compute` (one may
/// be more natural than the other in a given source language context).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Compute,
    Constrain,
}

/// Loop invariant declaration attached to a loop in LLZK IR.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InvariantDecl {
    pub loop_name: Identifier,
    pub bindings: Vec<Identifier>,
    pub body: Block,
    pub span: Span,
}

impl Visitable for InvariantDecl {}

/// Expression language used by contracts and predicates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expression {
    Conditional {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
        span: Span,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    Member {
        target: Box<Expression>,
        member: Identifier,
        span: Span,
    },
    Call {
        callee: Identifier,
        args: Vec<Expression>,
        span: Span,
    },
    Quantifier {
        quantifier_kind: QuantifierKind,
        binding: Identifier,
        domain: QuantifierDomain,
        body: Box<Expression>,
        span: Span,
    },
    Len {
        target: Box<Expression>,
        span: Span,
    },
    Old {
        expression: Box<Expression>,
        span: Span,
    },
    Arg {
        index: usize,
        span: Span,
    },
    Nondet {
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    Number {
        value: BigUint,
        span: Span,
    },
    Symbol(Identifier),
}

impl Expression {
    /// Returns the source span covering the current expression node.
    pub fn span(&self) -> Span {
        match self {
            Self::Conditional { span, .. }
            | Self::Binary { span, .. }
            | Self::Unary { span, .. }
            | Self::Index { span, .. }
            | Self::Member { span, .. }
            | Self::Call { span, .. }
            | Self::Quantifier { span, .. }
            | Self::Len { span, .. }
            | Self::Old { span, .. }
            | Self::Arg { span, .. }
            | Self::Nondet { span }
            | Self::Boolean { span, .. }
            | Self::Number { span, .. } => *span,
            Self::Symbol(ident) => ident.span,
        }
    }
}

impl Visitable for Expression {}

/// Supported quantifier kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantifierKind {
    Forall,
    Exists,
}

/// Domain over which a quantifier applies.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuantifierDomain {
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        span: Span,
    },
    Expr(Box<Expression>),
}

impl Visitable for QuantifierDomain {}

/// Binary operators recognized by the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    Pow,
}

/// Unary operators recognized by the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Not,
    Neg,
}
