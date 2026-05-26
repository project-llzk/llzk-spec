use std::ops::{Deref, DerefMut};

use crate::{
    ast::{BinaryOp, Expression, Spanned, UnaryOp, Visitable, Visitor},
    diagnostic::Diagnostic,
    type_analysis::{TypeSystem, TypingResult, error::TypeAnalysisError},
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
    /// Returns the expected type of the operands of this binary op.
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

    /// Returns the type of the binary op.
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
    /// Returns the expected type of the operands of this unary op.
    pub fn expected_type<T: TypeSystem>(&self, t: &mut T) -> T::Type {
        match self {
            UnaryOp::Not => t.bool_type(),
            UnaryOp::Neg => t.felt_type(),
        }
    }

    /// Returns the type of the unary op.
    pub fn return_type<T: TypeSystem>(&self, t: &mut T) -> T::Type {
        match self {
            UnaryOp::Not => t.bool_type(),
            UnaryOp::Neg => t.felt_type(),
        }
    }
}

/// Type-checks a sequence of entities using the same visitor.
///
/// The diagnostics emitted by the entities are grouped together. If any entity returns diagnostics
/// the whole sequence is considered a failure. However, other entities of the sequence are checked
/// as well in order to collect as many diagnostics as possible. If the checks pass then the
/// entities' results are combined using the `combine` callback.
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

/// Helper type for collecting diagnostics.
pub struct Diagnostics<'s> {
    diags: Vec<Diagnostic>,
    source_name: &'s str,
    spanned: &'s dyn Spanned,
}

impl<'s> Diagnostics<'s> {
    pub fn new(source_name: &'s str, spanned: &'s dyn Spanned) -> Self {
        Self {
            diags: Default::default(),
            source_name,
            spanned,
        }
    }
}

impl Diagnostics<'_> {
    /// Adds a typing error into the diagnostics set.
    pub fn add_type_err(&mut self, err: TypeAnalysisError, context: impl std::fmt::Display) {
        self.add_type_err_at_location(err, context, self.spanned)
    }

    /// Adds a typing error into the diagnostics set at the given location.
    pub fn add_type_err_at_location(
        &mut self,
        err: TypeAnalysisError,
        context: impl std::fmt::Display,
        spanned: &dyn Spanned,
    ) {
        let diag = err.into_diags(self.source_name, Some(spanned.span()), context);
        self.extend(diag)
    }

    /// Converts the given result into a typing result, adding the diagnostics accumulated so far
    /// to it.
    pub fn to_typing_result<R, C>(
        &mut self,
        r: Result<R, TypeAnalysisError>,
        context: impl FnOnce() -> C,
    ) -> TypingResult<R>
    where
        C: std::fmt::Display,
    {
        r.map_err(|err| {
            let mut new = Self {
                diags: self.diags.clone(),
                source_name: self.source_name,
                spanned: self.spanned,
            };
            new.add_type_err(err, context());
            new.into()
        })
    }

    /// Adds a new on-the-fly diagnostic.
    pub fn add(&mut self, message: impl std::fmt::Display) {
        self.add_at_location(message, self.spanned)
    }

    /// Adds a new on-the-fly diagnostic on the given location.
    pub fn add_at_location(&mut self, message: impl std::fmt::Display, spanned: &dyn Spanned) {
        self.diags.push(Diagnostic::new(
            self.source_name,
            message.to_string(),
            Some(spanned.span()),
        ))
    }

    /// Adds a new on-the-fly diagnostic if the given condition is false.
    pub fn add_unless<M: std::fmt::Display>(&mut self, check: bool, message: impl FnOnce() -> M) {
        if !check {
            self.add(message())
        }
    }

    /// Extracts the typing result, accumulating the diagnostics if the result was a failure.
    ///
    /// Return `Some` if the result was a `Ok`, returns `None` otherwise.
    pub fn extract_result<T>(&mut self, r: TypingResult<T>) -> Option<T> {
        match r {
            Ok(r) => Some(r),
            Err(e) => {
                self.diags.extend(e);
                None
            }
        }
    }

    /// Creates a typinh result from another result, accumulating the diagnostics if the result was a failure.
    pub fn from_other_result<T, E, M: std::fmt::Display>(
        &mut self,
        r: Result<T, E>,
        message: impl FnOnce(E) -> M,
    ) -> TypingResult<T> {
        r.map_err(|e| {
            let mut new = Self {
                diags: self.diags.clone(),
                source_name: self.source_name,
                spanned: self.spanned,
            };
            new.add(message(e));
            new.into()
        })
    }

    /// Extracts a typing error result, converting it into a diagnostic and accumulating it if the result was a failure.
    ///
    /// Return `Some` if the result was a `Ok`, returns `None` otherwise.
    pub fn extract_type_result<T, M: std::fmt::Display>(
        &mut self,
        r: Result<T, TypeAnalysisError>,
        context: impl FnOnce() -> M,
    ) -> Option<T> {
        match r {
            Ok(r) => Some(r),
            Err(e) => {
                self.add_type_err(e, context());
                None
            }
        }
    }

    /// Extracts many typing results, accumulating the diagnostics if any result was a failure.
    ///
    /// For each result that returned `Ok` returns `Some`, `None` otherwise.
    pub fn extract_many_results<T>(
        &mut self,
        rs: impl IntoIterator<Item = TypingResult<T>>,
    ) -> Vec<Option<T>> {
        rs.into_iter().map(|r| self.extract_result(r)).collect()
    }

    /// Convert the diagnostics into a typing result.
    ///
    /// If diagnostics have been collected, returns `Err` with them. Otherwise returns the
    /// result of the `ok` callback.
    pub fn finish<R>(self, ok: impl FnOnce() -> R) -> TypingResult<R> {
        if !self.diags.is_empty() {
            return Err(self.diags);
        }

        Ok(ok())
    }
}

impl DerefMut for Diagnostics<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.diags
    }
}

impl Deref for Diagnostics<'_> {
    type Target = Vec<Diagnostic>;

    fn deref(&self) -> &Self::Target {
        &self.diags
    }
}

impl From<Diagnostics<'_>> for Vec<Diagnostic> {
    fn from(diags: Diagnostics) -> Self {
        diags.diags
    }
}
