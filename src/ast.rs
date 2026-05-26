//! Abstract syntax tree for `llzk-spec`.

use internment::ArenaIntern;
use num_bigint::BigUint;
use serde::Serialize;

pub mod ctx;

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
impl<V: Visitable> Visitable for Vec<V> {}

/// Source location of an AST node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

/// Trait for AST entities that have a [`Span`]
pub trait Spanned {
    fn span(&self) -> Span;
}

/// Identifier with attached source span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identifier<'a, M = ()> {
    name: Symbol<'a>,
    span: Span,
    meta: M,
}

impl<'a, M> Identifier<'a, M> {
    pub fn value(&self) -> &'a str {
        self.name.value()
    }

    pub fn symbol(&self) -> Symbol<'a> {
        self.name
    }

    pub fn meta(&self) -> &M {
        &self.meta
    }
}

impl<'a> Identifier<'a> {
    pub fn new(name: Symbol<'a>, span: Span) -> Self {
        Self {
            name,
            span,
            meta: (),
        }
    }

    pub fn with_meta<M>(&self, meta: M) -> Identifier<'a, M> {
        Identifier {
            name: self.name,
            span: self.span,
            meta,
        }
    }
}

impl<M> AsRef<str> for Identifier<'_, M> {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl<M> Spanned for Identifier<'_, M> {
    fn span(&self) -> Span {
        self.span
    }
}

/// Parsed top-level document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Document<'a, M = ()> {
    items: Vec<Item<'a, M>>,
    span: Span,
}

impl<'a, M> Document<'a, M> {
    pub fn new(items: Vec<Item<'a, M>>, span: Span) -> Self {
        Self { items, span }
    }

    pub fn items(&self) -> &[Item<'a, M>] {
        &self.items
    }
}

impl<M> Spanned for Document<'_, M> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<M> Visitable for Document<'_, M> {}

impl<'d, 'a, M> IntoIterator for &'d Document<'a, M> {
    type Item = &'d Item<'a, M>;

    type IntoIter = <&'d Vec<Item<'a, M>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

/// Top-level declarations accepted by the language.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item<'a, M = ()> {
    Contract(ContractDecl<'a, M>),
    Predicate(PredicateDecl<'a, M>),
}

impl<'a, M> From<ContractDecl<'a, M>> for Item<'a, M> {
    fn from(value: ContractDecl<'a, M>) -> Self {
        Self::Contract(value)
    }
}

impl<'a, M> From<PredicateDecl<'a, M>> for Item<'a, M> {
    fn from(value: PredicateDecl<'a, M>) -> Self {
        Self::Predicate(value)
    }
}

impl<M> Spanned for Item<'_, M> {
    fn span(&self) -> Span {
        match self {
            Item::Contract(decl) => decl.span(),
            Item::Predicate(decl) => decl.span(),
        }
    }
}

impl<M> Visitable for Item<'_, M> {}

/// Contract declaration attached to an LLZK symbol.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDecl<'a, M = ()> {
    target: Identifier<'a, M>,
    body: Block<'a, M>,
    span: Span,
}

impl<'a, M> ContractDecl<'a, M> {
    pub fn new(target: Identifier<'a, M>, body: Block<'a, M>, span: Span) -> Self {
        Self { target, body, span }
    }

    pub fn target(&self) -> &Identifier<'a, M> {
        &self.target
    }

    pub fn body(&self) -> &Block<'a, M> {
        &self.body
    }
}

impl<M> Spanned for ContractDecl<'_, M> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<M> Visitable for ContractDecl<'_, M> {}

/// Predicate declaration in block or inline-expression form.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredicateDecl<'a, M = ()> {
    name: Identifier<'a, M>,
    params: Vec<Identifier<'a, M>>,
    body: Block<'a, M>,
    span: Span,
}

impl<'a, M> PredicateDecl<'a, M> {
    pub fn new(
        name: Identifier<'a, M>,
        params: impl IntoIterator<Item = Identifier<'a, M>>,
        body: Block<'a, M>,
        span: Span,
    ) -> Self {
        Self {
            name,
            params: Vec::from_iter(params),
            body,
            span,
        }
    }

    pub fn name(&self) -> &Identifier<'a, M> {
        &self.name
    }

    pub fn body(&self) -> &Block<'a, M> {
        &self.body
    }

    pub fn params(&self) -> &[Identifier<'a, M>] {
        &self.params
    }
}

impl<M> Spanned for PredicateDecl<'_, M> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<M> Visitable for PredicateDecl<'_, M> {}

/// Sequence of statements with a lexical scope.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block<'a, M = ()> {
    statements: Vec<Statement<'a, M>>,
    span: Span,
}

impl<'a, M> Block<'a, M> {
    pub fn new(statements: impl IntoIterator<Item = Statement<'a, M>>, span: Span) -> Self {
        Self {
            statements: Vec::from_iter(statements),
            span,
        }
    }

    pub fn statements(&self) -> &[Statement<'a, M>] {
        &self.statements
    }
}

impl<M> Spanned for Block<'_, M> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'b, 'a, M> IntoIterator for &'b Block<'a, M> {
    type Item = &'b Statement<'a, M>;

    type IntoIter = <&'b Vec<Statement<'a, M>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.statements.iter()
    }
}

impl<M> Visitable for Block<'_, M> {}

/// Statements supported by phase 1 of `llzk-spec`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Statement<'a, M = ()> {
    Scoped {
        scope: Scope,
        statement: Box<Statement<'a, M>>,
        span: Span,
    },
    Block(Block<'a, M>),
    Require {
        expression: Expression<'a, M>,
        span: Span,
    },
    Ensure {
        expression: Expression<'a, M>,
        span: Span,
    },
    Let {
        name: Identifier<'a, M>,
        value: Expression<'a, M>,
        span: Span,
    },
    Unused {
        name: Identifier<'a, M>,
        span: Span,
    },
    Return {
        expression: Expression<'a, M>,
        span: Span,
    },
    Increases {
        expression: Expression<'a, M>,
        span: Span,
    },
    Decreases {
        expression: Expression<'a, M>,
        span: Span,
    },
    Step {
        expression: Expression<'a, M>,
        span: Span,
    },
    Invariant(InvariantDecl<'a, M>),
    Predicate(PredicateDecl<'a, M>),
    Empty {
        span: Span,
    },
}

impl<M> Spanned for Statement<'_, M> {
    fn span(&self) -> Span {
        match self {
            Statement::Block(block) => block.span,
            Statement::Scoped { span, .. }
            | Statement::Require { span, .. }
            | Statement::Ensure { span, .. }
            | Statement::Let { span, .. }
            | Statement::Unused { span, .. }
            | Statement::Return { span, .. }
            | Statement::Increases { span, .. }
            | Statement::Decreases { span, .. }
            | Statement::Step { span, .. }
            | Statement::Empty { span } => *span,
            Statement::Invariant(decl) => decl.span,
            Statement::Predicate(decl) => decl.span,
        }
    }
}

impl<M> Visitable for Statement<'_, M> {}

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
pub struct InvariantDecl<'a, M = ()> {
    loop_name: Identifier<'a, M>,
    bindings: Vec<Identifier<'a, M>>,
    body: Block<'a, M>,
    span: Span,
}

impl<'a, M> InvariantDecl<'a, M> {
    pub fn new(
        loop_name: Identifier<'a, M>,
        bindings: impl IntoIterator<Item = Identifier<'a, M>>,
        body: Block<'a, M>,
        span: Span,
    ) -> Self {
        Self {
            loop_name,
            bindings: Vec::from_iter(bindings),
            body,
            span,
        }
    }

    pub fn loop_name(&self) -> &Identifier<'a, M> {
        &self.loop_name
    }

    pub fn body(&self) -> &Block<'a, M> {
        &self.body
    }

    pub fn bindings(&self) -> &[Identifier<'a, M>] {
        &self.bindings
    }
}
impl<M> Spanned for InvariantDecl<'_, M> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<M> Visitable for InvariantDecl<'_, M> {}

/// Expression language used by contracts and predicates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expression<'a, M = ()> {
    Conditional {
        condition: Box<Expression<'a, M>>,
        then_branch: Box<Expression<'a, M>>,
        else_branch: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression<'a, M>>,
        right: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Index {
        target: Box<Expression<'a, M>>,
        index: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Member {
        target: Box<Expression<'a, M>>,
        member: Identifier<'a, M>,
        span: Span,
        meta: M,
    },
    Call {
        callee: Identifier<'a, M>,
        args: Vec<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Quantifier {
        quantifier_kind: QuantifierKind,
        binding: Identifier<'a, M>,
        domain: QuantifierDomain<'a, M>,
        body: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Len {
        target: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Old {
        expression: Box<Expression<'a, M>>,
        span: Span,
        meta: M,
    },
    Arg {
        index: usize,
        span: Span,
        meta: M,
    },
    Nondet {
        span: Span,
        meta: M,
    },
    Boolean {
        value: bool,
        span: Span,
        meta: M,
    },
    Number {
        value: Literal<'a>,
        span: Span,
        meta: M,
    },
    Symbol(Identifier<'a, M>),
}

impl<M> Expression<'_, M> {
    /// Returns the metadata of the expression.
    pub fn meta(&self) -> &M {
        match self {
            Self::Conditional { meta, .. }
            | Self::Binary { meta, .. }
            | Self::Unary { meta, .. }
            | Self::Index { meta, .. }
            | Self::Member { meta, .. }
            | Self::Call { meta, .. }
            | Self::Quantifier { meta, .. }
            | Self::Len { meta, .. }
            | Self::Old { meta, .. }
            | Self::Arg { meta, .. }
            | Self::Nondet { meta, .. }
            | Self::Boolean { meta, .. }
            | Self::Number { meta, .. } => meta,
            Self::Symbol(ident) => &ident.meta,
        }
    }
}

impl<M> Spanned for Expression<'_, M> {
    fn span(&self) -> Span {
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
            | Self::Nondet { span, .. }
            | Self::Boolean { span, .. }
            | Self::Number { span, .. } => *span,
            Self::Symbol(ident) => ident.span,
        }
    }
}

impl<M> Visitable for Expression<'_, M> {}

/// Supported quantifier kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantifierKind {
    Forall,
    Exists,
}

impl std::fmt::Display for QuantifierKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if f.alternate() {
                match self {
                    QuantifierKind::Forall => "∀",
                    QuantifierKind::Exists => "∃",
                }
            } else {
                match self {
                    QuantifierKind::Forall => "forall",
                    QuantifierKind::Exists => "exists",
                }
            }
        )
    }
}

/// Domain over which a quantifier applies.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuantifierDomain<'a, M = ()> {
    Range {
        start: Box<Expression<'a, M>>,
        end: Box<Expression<'a, M>>,
        span: Span,
    },
    Expr(Box<Expression<'a, M>>),
}

impl<M> Spanned for QuantifierDomain<'_, M> {
    fn span(&self) -> Span {
        match self {
            QuantifierDomain::Range { span, .. } => *span,
            QuantifierDomain::Expr(expression) => expression.span(),
        }
    }
}

impl<M> Visitable for QuantifierDomain<'_, M> {}

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

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryOp::Or => "||",
                BinaryOp::And => "&&",
                BinaryOp::Eq => "==",
                BinaryOp::Ne => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Mod => "%",
                BinaryOp::BitAnd => "&",
                BinaryOp::Pow => "**",
            }
        )
    }
}

/// Unary operators recognized by the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Not,
    Neg,
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            }
        )
    }
}

/// Interned symbol in the AST context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
