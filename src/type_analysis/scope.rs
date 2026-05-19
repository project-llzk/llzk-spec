//! Types and functions for handling lexical scopes.

use std::collections::HashMap;

use crate::{
    ast::{Identifier, Symbol},
    type_analysis::error::TypeAnalysisError,
};

pub(super) struct ScopeStack<'ast, L, F> {
    root: Scope<'ast, L, F>,
    scopes: Vec<Scope<'ast, L, F>>,
}

impl<'ast, L, F> ScopeStack<'ast, L, F> {
    pub fn new() -> Self {
        Self {
            root: Scope::new(true),
            scopes: vec![],
        }
    }

    pub fn top(&mut self) -> &mut Scope<'ast, L, F> {
        self.scopes.last_mut().unwrap_or(&mut self.root)
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::new(false))
    }

    pub fn push_local_limit(&mut self) {
        self.scopes.push(Scope::new(true))
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    pub fn find_local(&self, name: &Identifier<'ast>) -> Result<&L, TypeAnalysisError> {
        self.ordered_scopes()
            // Only check scopes that are within the local limit.
            .take_while(|scope| !scope.local_limit)
            // Fetch the closest binding
            .find_map(|scope| scope.locals.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownLocal(name.value().to_owned()))
    }

    pub fn find_predicate(&self, name: &Identifier<'ast>) -> Result<&F, TypeAnalysisError> {
        self.ordered_scopes()
            .find_map(|scope| scope.predicates.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownPredicate(name.value().to_owned()))
    }

    /// Returns an iterator of the scopes in stack order top to bottom.
    fn ordered_scopes(&self) -> impl Iterator<Item = &Scope<'ast, L, F>> {
        self.scopes.iter().rev().chain([&self.root])
    }
}

/// Entry in the scope stack.
pub(super) struct Scope<'ast, L, F> {
    // Binds names to predicates.
    predicates: HashMap<Symbol<'ast>, F>,
    // Binds local names to SSA values.
    locals: HashMap<Symbol<'ast>, L>,
    // Indicates wether this scope entry limits the access to the locals defined in outer scopes.
    local_limit: bool,
}

impl<'ast, L, F> Scope<'ast, L, F> {
    fn new(local_limit: bool) -> Self {
        Self {
            predicates: Default::default(),
            locals: Default::default(),
            local_limit,
        }
    }

    pub fn bind_predicate(
        &mut self,
        name: &Identifier<'ast>,
        f: F,
    ) -> Result<(), TypeAnalysisError> {
        if self.predicates.contains_key(&name.symbol()) {
            return Err(TypeAnalysisError::DuplicatePredicate(
                name.value().to_owned(),
            ));
        }
        self.predicates.insert(name.symbol(), f);
        Ok(())
    }

    pub fn bind_local(&mut self, name: &Identifier<'ast>, l: L) -> Result<(), TypeAnalysisError> {
        if self.locals.contains_key(&name.symbol()) {
            return Err(TypeAnalysisError::DuplicateLocal(name.value().to_owned()));
        }
        self.locals.insert(name.symbol(), l);
        Ok(())
    }
}
