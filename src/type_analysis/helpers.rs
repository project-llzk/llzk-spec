use crate::{
    ast::{BinaryOp, Expression, UnaryOp, Visitable, Visitor},
    diagnostic::Diagnostic,
    type_analysis::{TypeSystem, TypingResult},
};

impl<'ast, T: Clone + PartialEq> Expression<'ast, T> {
    /// Returns the type of the expression.
    pub fn r#type(&self) -> T {
        self.meta().clone()
    }

    /// Returns true if the expression is of the given type.
    pub fn has_type(&self, r#type: T) -> bool {
        self.r#type() == r#type
    }
}

impl BinaryOp {
    pub fn expected_type<T: TypeSystem>(&self, t: &mut T) -> T::Type {
        match self {
            BinaryOp::Or | BinaryOp::And => t.bool_type(),
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
            | BinaryOp::Pow => t.felt_type(),
        }
    }

    pub fn return_type<T: TypeSystem>(&self, t: &mut T) -> T::Type {
        match self {
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => t.bool_type(),
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::BitAnd
            | BinaryOp::Pow => t.felt_type(),
        }
    }
}

impl UnaryOp {
    pub fn expected_type<T: TypeSystem>(&self, t: &mut T) -> T::Type {
        match self {
            UnaryOp::Not => t.bool_type(),
            UnaryOp::Neg => t.felt_type(),
        }
    }

    pub fn return_type<T: TypeSystem>(&self, t: &mut T) -> T::Type {
        match self {
            UnaryOp::Not => t.bool_type(),
            UnaryOp::Neg => t.felt_type(),
        }
    }
}

pub fn extract_result<T>(r: TypingResult<T>, diags: &mut Vec<Diagnostic>) -> Option<T> {
    match r {
        Ok(r) => Some(r),
        Err(e) => {
            diags.extend(e);
            None
        }
    }
}

pub fn check_many<'a, V, I, O, E, R>(
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
