use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Document {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    Contract(ContractDecl),
    Predicate(PredicateDecl),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDecl {
    pub target: Identifier,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredicateDecl {
    pub name: Identifier,
    pub params: Vec<Identifier>,
    pub body: PredicateBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PredicateBody {
    Block(Block),
    Expr(Expression),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

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
    Invariant(InvariantDecl),
    Predicate(PredicateDecl),
    Empty {
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Compute,
    Witness,
    Constrain,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InvariantDecl {
    pub loop_label: Identifier,
    pub induction_var: Identifier,
    pub body: Block,
    pub span: Span,
}

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
    Nondet {
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    Number {
        value: String,
        span: Span,
    },
    Symbol(Identifier),
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Self::Conditional { span, .. }
            | Self::Binary { span, .. }
            | Self::Unary { span, .. }
            | Self::Index { span, .. }
            | Self::Call { span, .. }
            | Self::Quantifier { span, .. }
            | Self::Len { span, .. }
            | Self::Nondet { span }
            | Self::Boolean { span, .. }
            | Self::Number { span, .. } => *span,
            Self::Symbol(ident) => ident.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantifierKind {
    Forall,
    Exists,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Not,
    Neg,
}
