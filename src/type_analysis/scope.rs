//! Types and functions for handling lexical scopes.

use std::collections::HashMap;

use crate::{
    ast::{Identifier, Symbol},
    type_analysis::{
        FnTypeProperties, TypeProperties, TypeSystem, ctx::Subst, error::TypeAnalysisError,
    },
};

pub(super) struct ScopeStack<'ast, L, F> {
    root: Scope<'ast, L, F>,
    scopes: Vec<Scope<'ast, L, F>>,
}

impl<'ast, L, F> ScopeStack<'ast, L, F> {
    /// Creates a ready to use stack of scopes with a root scope.
    ///
    /// There is no need of pushing a scope before using the stack.
    pub fn new() -> Self {
        Self {
            root: Scope::new(true),
            scopes: vec![],
        }
    }

    /// Returns a reference to the top of the stack.
    pub fn top(&mut self) -> &mut Scope<'ast, L, F> {
        self.scopes.last_mut().unwrap_or(&mut self.root)
    }

    /// Pushes a new scope.
    pub fn push(&mut self) {
        self.scopes.push(Scope::new(false))
    }

    /// Pushes a new scope that is tagged as a local limit.
    ///
    /// Scopes tagged as local limits act as barriers when looking up local bindings, hiding any
    /// locals beyond the limit from the lookup.
    pub fn push_local_limit(&mut self) {
        self.scopes.push(Scope::new(true))
    }

    /// Pops the top scope.
    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    /// Looks for a local binding with the given identifier.
    pub fn find_local(&self, name: &Identifier<'ast>) -> Result<&L, TypeAnalysisError> {
        self.ordered_local_scopes()
            // Fetch the closest binding
            .find_map(|scope| scope.locals.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownLocal(name.value().to_owned()))
    }

    /// Looks for a predicate definition with the given identifier.
    pub fn find_predicate(&self, name: &Identifier<'ast>) -> Result<&F, TypeAnalysisError> {
        self.ordered_scopes()
            .find_map(|scope| scope.predicates.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownPredicate(name.value().to_owned()))
    }

    /// Returns an iterator of the scopes in stack order top to bottom.
    fn ordered_scopes(&self) -> impl Iterator<Item = &Scope<'ast, L, F>> {
        self.scopes.iter().rev().chain([&self.root])
    }

    /// Returns an iterator of the scopes in stack order top to bottom until a local limit is
    /// reached.
    fn ordered_local_scopes(&self) -> impl Iterator<Item = &Scope<'ast, L, F>> {
        struct Iter<I> {
            it: I,
            limit_reached: bool,
        }

        impl<'s, 'ast, L, F, I> Iterator for Iter<I>
        where
            I: Iterator<Item = &'s Scope<'ast, L, F>>,
            'ast: 's,
            L: 's,
            F: 's,
        {
            type Item = &'s Scope<'ast, L, F>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.limit_reached {
                    return None;
                }
                let next = self.it.next()?;
                self.limit_reached = next.local_limit;
                Some(next)
            }
        }

        Iter {
            it: self.ordered_scopes(),
            limit_reached: false,
        }
    }

    /// Returns an iterator of mutable references to the scopes in stack order top to bottom.
    fn ordered_scopes_mut(&mut self) -> impl Iterator<Item = &mut Scope<'ast, L, F>> {
        self.scopes.iter_mut().rev().chain([&mut self.root])
    }

    /// Propagates resolved types across type variables binded in the scope.
    pub fn propagate<T>(&mut self, subst: &Subst<T>, ts: &mut T)
    where
        T: TypeSystem<Type = L, FnType = F>,
        L: TypeProperties + Clone,
        F: FnTypeProperties<Type = L>,
    {
        self.all_local_types()
            .filter(|l| l.is_var_type())
            .for_each(|l| {
                let id = l.var_id().unwrap();
                if let Some(r) = subst.get(&id) {
                    *l = r.clone();
                }
            });
        self.all_predicate_types()
            .filter(|f| f.contains_type_vars())
            .for_each(|f| {
                let mut ins = f.inputs().to_vec();
                let mut outs = f.outputs().to_vec();
                for i in ins.iter_mut() {
                    let _ = propagate_type::<T, L>(i, subst);
                }
                for o in outs.iter_mut() {
                    let _ = propagate_type::<T, L>(o, subst);
                }
                let r = ts.func_type(&ins, &outs);
                *f = r;
            });

        fn propagate_type<T, L>(l: &mut L, subst: &Subst<T>) -> Option<()>
        where
            T: TypeSystem<Type = L>,
            L: TypeProperties + Clone,
        {
            let id = l.var_id()?;
            let r = subst.get(&id)?;
            *l = r.clone();
            Some(())
        }
    }

    /// Returns an iterator of mutable references to all the type bindings of locals.
    fn all_local_types(&mut self) -> impl Iterator<Item = &mut L> {
        self.ordered_scopes_mut()
            .flat_map(|scope| scope.locals.values_mut())
    }

    /// Returns an iterator of mutable references to all the type bindings of predicates.
    fn all_predicate_types(&mut self) -> impl Iterator<Item = &mut F> {
        self.ordered_scopes_mut()
            .flat_map(|scope| scope.predicates.values_mut())
    }
}

impl<L, F> std::fmt::Debug for ScopeStack<'_, L, F>
where
    L: std::fmt::Display,
    F: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope_count = self.scopes.len() + 1;
        self.ordered_scopes().enumerate().try_for_each(|(n, s)| {
            let n = scope_count - n;
            writeln!(f, "Scope #{n}")?;
            std::fmt::Debug::fmt(s, f)
        })
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
    /// Creates a new scope.
    fn new(local_limit: bool) -> Self {
        Self {
            predicates: Default::default(),
            locals: Default::default(),
            local_limit,
        }
    }

    /// Binds the given value to a name in the predicates namespace.
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

    /// Binds the given value to a name in the locals namespace.
    pub fn bind_local(&mut self, name: &Identifier<'ast>, l: L) -> Result<(), TypeAnalysisError> {
        if self.locals.contains_key(&name.symbol()) {
            return Err(TypeAnalysisError::DuplicateLocal(name.value().to_owned()));
        }
        self.locals.insert(name.symbol(), l);
        Ok(())
    }
}

impl<L, F> std::fmt::Debug for Scope<'_, L, F>
where
    L: std::fmt::Display,
    F: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Predicates:")?;
        self.predicates
            .iter()
            .try_for_each(|(symbol, binding)| writeln!(f, "    {}: {binding}", symbol.value()))?;
        writeln!(f, "  Locals:")?;
        self.locals
            .iter()
            .try_for_each(|(symbol, binding)| writeln!(f, "    {}: {binding}", symbol.value()))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{AstContext, Span};

    use super::*;

    type Scopes<'ast> = ScopeStack<'ast, usize, usize>;

    fn ctx() -> AstContext {
        AstContext::new()
    }

    fn ident<'ast>(ctx: &'ast AstContext, symbol: &str) -> Identifier<'ast> {
        Identifier::new(ctx.symbol(symbol), Span::default())
    }

    #[test]
    fn test_local_limits_1() {
        let ctx = ctx();
        let x = ident(&ctx, "x");
        let y = ident(&ctx, "y");
        let mut stack = Scopes::new();

        // Test that 'x' can be accessed but 'y' cannot
        // when 'x' is within the limit and 'y' is outside.
        stack.top().bind_local(&y, 1).unwrap();
        stack.push_local_limit();
        stack.push();
        stack.top().bind_local(&x, 2).unwrap();

        assert_eq!(stack.find_local(&x), Ok(&2));
        assert_eq!(
            stack.find_local(&y),
            Err(TypeAnalysisError::UnknownLocal("y".to_string()))
        )
    }

    #[test]
    fn test_local_limits_2() {
        let ctx = ctx();
        let x = ident(&ctx, "x");
        let y = ident(&ctx, "y");
        let mut stack = Scopes::new();

        // Test that 'x' can be accessed but 'y' cannot
        // when 'x' is right at the limit and 'y' is outside.
        stack.top().bind_local(&y, 1).unwrap();
        stack.push_local_limit();
        stack.top().bind_local(&x, 2).unwrap();

        assert_eq!(stack.find_local(&x), Ok(&2));
        assert_eq!(
            stack.find_local(&y),
            Err(TypeAnalysisError::UnknownLocal("y".to_string()))
        )
    }

    #[test]
    fn test_shadowing() {
        let ctx = ctx();
        let x = ident(&ctx, "x");
        let mut stack = Scopes::new();

        // Test that when accesing 'x' we get the closest result back.
        stack.top().bind_local(&x, 1).unwrap();
        stack.push();
        stack.top().bind_local(&x, 2).unwrap();

        assert_eq!(stack.find_local(&x), Ok(&2));
    }
}
