//! Abstract syntax tree for `llzk-spec`.

use internment::ArenaIntern;
use num_bigint::BigUint;
use serde::Serialize;

pub mod ctx;
pub mod type_analysis;

pub use ctx::AstContext;

/// Trait defining a visitor of AST entities.
pub trait Visitor<E: Visitable> {
    type Output;

    fn visit(&mut self, entity: &E) -> Self::Output;
}

impl<E: Visitable, V: Visitor<E>> Visitor<Box<E>> for V {
    type Output = <V as Visitor<E>>::Output;

    fn visit(&mut self, entity: &Box<E>) -> Self::Output {
        entity.as_ref().accept(self)
    }
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

impl<V: Visitable> Visitable for Box<V> {}

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
pub struct Identifier<'a> {
    pub name: Symbol<'a>,
    pub span: Span,
}

impl AsRef<str> for Identifier<'_> {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

/// Parsed top-level document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Document<'a> {
    pub items: Vec<Item<'a>>,
    pub span: Span,
}

impl Visitable for Document<'_> {}

/// Top-level declarations accepted by the language.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item<'a> {
    Contract(ContractDecl<'a>),
    Predicate(PredicateDecl<'a>),
}

impl Visitable for Item<'_> {}

/// Contract declaration attached to an LLZK symbol.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDecl<'a> {
    pub target: Identifier<'a>,
    pub body: Block<'a>,
    pub span: Span,
}

impl Visitable for ContractDecl<'_> {}

/// Predicate declaration in block or inline-expression form.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredicateDecl<'a> {
    pub name: Identifier<'a>,
    pub params: Vec<Identifier<'a>>,
    pub body: Block<'a>,
    pub span: Span,
}

impl Visitable for PredicateDecl<'_> {}

/// Sequence of statements with a lexical scope.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block<'a> {
    pub statements: Vec<Statement<'a>>,
    pub span: Span,
}

impl Visitable for Block<'_> {}

/// Statement<'a>s supported by phase 1 of `llzk-spec`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Statement<'a> {
    Scoped {
        scope: Scope,
        statement: Box<Statement<'a>>,
        span: Span,
    },
    Block(Block<'a>),
    Require {
        expression: Expression<'a>,
        span: Span,
    },
    Ensure {
        expression: Expression<'a>,
        span: Span,
    },
    Let {
        name: Identifier<'a>,
        value: Expression<'a>,
        span: Span,
    },
    Unused {
        name: Identifier<'a>,
        span: Span,
    },
    Return {
        expression: Expression<'a>,
        span: Span,
    },
    Increases {
        expression: Expression<'a>,
        span: Span,
    },
    Decreases {
        expression: Expression<'a>,
        span: Span,
    },
    Step {
        expression: Expression<'a>,
        span: Span,
    },
    Invariant(InvariantDecl<'a>),
    Predicate(PredicateDecl<'a>),
    Empty {
        span: Span,
    },
}

impl Visitable for Statement<'_> {}

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
pub struct InvariantDecl<'a> {
    pub loop_name: Identifier<'a>,
    pub bindings: Vec<Identifier<'a>>,
    pub body: Block<'a>,
    pub span: Span,
}

impl Visitable for InvariantDecl<'_> {}

/// Expression language used by contracts and predicates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expression<'a> {
    Conditional {
        condition: Box<Expression<'a>>,
        then_branch: Box<Expression<'a>>,
        else_branch: Box<Expression<'a>>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression<'a>>,
        span: Span,
    },
    Index {
        target: Box<Expression<'a>>,
        index: Box<Expression<'a>>,
        span: Span,
    },
    Member {
        target: Box<Expression<'a>>,
        member: Identifier<'a>,
        span: Span,
    },
    Call {
        callee: Identifier<'a>,
        args: Vec<Expression<'a>>,
        span: Span,
    },
    Quantifier {
        quantifier_kind: QuantifierKind,
        binding: Identifier<'a>,
        domain: QuantifierDomain<'a>,
        body: Box<Expression<'a>>,
        span: Span,
    },
    Len {
        target: Box<Expression<'a>>,
        span: Span,
    },
    Old {
        expression: Box<Expression<'a>>,
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
        value: Literal<'a>,
        span: Span,
    },
    Symbol(Identifier<'a>),
}

impl Expression<'_> {
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

impl Visitable for Expression<'_> {}

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
pub enum QuantifierDomain<'a> {
    Range {
        start: Box<Expression<'a>>,
        end: Box<Expression<'a>>,
        span: Span,
    },
    Expr(Box<Expression<'a>>),
}

impl Visitable for QuantifierDomain<'_> {}

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

/// Interned symbol in the AST context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol<'a> {
    inner: ArenaIntern<'a, str>,
}

impl<'a> Symbol<'a> {
    pub fn value(&self) -> &'a str {
        self.inner.into_ref()
    }
}

impl AsRef<str> for Symbol<'_> {
    fn as_ref(&self) -> &str {
        self.value()
    }
}

impl Serialize for Symbol<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.value())
    }
}

impl PartialEq<str> for Symbol<'_> {
    fn eq(&self, other: &str) -> bool {
        self.value() == other
    }
}

impl PartialEq<&str> for Symbol<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.value() == *other
    }
}

impl std::fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}

impl From<Symbol<'_>> for String {
    fn from(value: Symbol<'_>) -> Self {
        value.value().to_owned()
    }
}

/// Interned big integer in the AST context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Literal<'a> {
    inner: ArenaIntern<'a, BigUint>,
}

impl<'a> Literal<'a> {
    pub fn value(&self) -> &'a BigUint {
        self.inner.into_ref()
    }
}

impl AsRef<BigUint> for Literal<'_> {
    fn as_ref(&self) -> &BigUint {
        self.value()
    }
}

impl Serialize for Literal<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.value().serialize(serializer)
    }
}

impl std::fmt::Display for Literal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}
