//! Types and functions for handling lexical scopes.

use std::collections::HashMap;

use crate::{
    ast::{Identifier, Symbol},
    type_analysis::{
        FnTypeProperties, TypeProperties, TypeSystem, ctx::Subst, error::TypeAnalysisError,
        loops::LoopInfo,
    },
};

/// Stack of lexical scopes used during type-checking.
///
/// This type is meant to be reusable in places that require keeping track of the lexical scoping
/// rules of the language. For that purpose, what is actually bound to the names is parametrized by
/// `L` and `F`. `L` is the type used for local names and `F` the type used for function-like
/// entities like predicates. The parameter `P` represents an additional payload type that is appended to
/// each scope that clients can use for augmenting them with data specific to their use case.
pub struct ScopeStack<'ast, L, F, P = ()> {
    scopes: Vec<Scope<'ast, L, F, P>>,
}

impl<'ast, L, F, P> ScopeStack<'ast, L, F, P> {
    /// Creates a ready to use stack of scopes with a root scope.
    ///
    /// There is no need of pushing a scope before using the stack.
    pub fn new(root_payload: P) -> Self {
        Self {
            scopes: vec![Scope::new(true, root_payload)],
        }
    }

    /// Returns a reference to the top of the stack.
    pub fn top(&mut self) -> &mut Scope<'ast, L, F, P> {
        self.scopes.last_mut().expect("at least one scope")
    }

    /// Pushes a new scope.
    pub fn push(&mut self, payload: P) {
        self.scopes.push(Scope::new(false, payload))
    }

    /// Pushes a new scope that is tagged as a local limit.
    ///
    /// Scopes tagged as local limits act as barriers when looking up local bindings, hiding any
    /// locals beyond the limit from the lookup.
    pub fn push_local_limit(&mut self, payload: P) {
        self.scopes.push(Scope::new(true, payload))
    }

    /// Pops the top scope.
    ///
    /// # Panics
    ///
    /// If attempted to pop the root scope.
    pub fn pop(&mut self) {
        assert!(self.scopes.len() > 1, "cannot pop the root scope");
        self.scopes.pop();
    }

    /// Looks for a local binding with the given identifier.
    pub fn find_local<M>(&self, name: &Identifier<'ast, M>) -> Result<&L, TypeAnalysisError> {
        self.ordered_local_scopes()
            // Fetch the closest binding
            .find_map(|scope| scope.locals.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownLocal(name.value().to_owned()))
    }

    /// Looks for a local binding with the given parameter number.
    pub fn find_parameter(&self, param_no: &usize) -> Result<&L, TypeAnalysisError> {
        self.ordered_local_scopes()
            // Fetch the closest binding
            .find_map(|scope| {
                let sym = scope.params.get(param_no)?;
                scope.locals.get(&sym)
            })
            .ok_or_else(|| TypeAnalysisError::UnknownLocal(format!("argument #{param_no}")))
    }

    /// Looks for a local binding with the given output number.
    pub fn find_output(&self, output_no: &usize) -> Result<&L, TypeAnalysisError> {
        self.ordered_local_scopes()
            // Fetch the closest binding
            .find_map(|scope| {
                let sym = scope.outputs.get(output_no)?;
                scope.locals.get(&sym)
            })
            .ok_or_else(|| TypeAnalysisError::UnknownLocal(format!("output #{output_no}")))
    }

    /// Looks for a predicate definition with the given identifier.
    pub fn find_predicate<M>(&self, name: &Identifier<'ast, M>) -> Result<&F, TypeAnalysisError> {
        self.ordered_scopes()
            .find_map(|scope| scope.predicates.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownPredicate(name.value().to_owned()))
    }

    /// Returns an iterator of the scopes in stack order top (most nested scope) to bottom (root scope).
    pub fn ordered_scopes(&self) -> impl Iterator<Item = &Scope<'ast, L, F, P>> {
        self.scopes.iter().rev()
    }

    /// Returns an iterator of the scopes in stack order top (most nested scope) to bottom (root scope) until a local limit is
    /// reached.
    pub fn ordered_local_scopes(&self) -> impl Iterator<Item = &Scope<'ast, L, F, P>> {
        struct Iter<I> {
            it: I,
            limit_reached: bool,
        }

        impl<'s, 'ast, L, F, P, I> Iterator for Iter<I>
        where
            I: Iterator<Item = &'s Scope<'ast, L, F, P>>,
            'ast: 's,
            L: 's,
            F: 's,
            P: 's,
        {
            type Item = &'s Scope<'ast, L, F, P>;

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

    /// Returns an iterator of mutable references to the scopes in stack order top (most nested scope) to bottom (root scope).
    pub fn ordered_scopes_mut(&mut self) -> impl Iterator<Item = &mut Scope<'ast, L, F, P>> {
        self.scopes.iter_mut().rev()
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

impl<'ast, L, F> ScopeStack<'ast, L, F, ()> {
    /// Propagates resolved types across type variables bound in the scope.
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
        self.all_predicate_types().for_each(|f| {
            if !f.contains_type_vars() {
                return;
            }
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

    /// Looks for a loop binding with the given identifier.
    pub fn find_loop(&self, name: &Identifier<'ast>) -> Result<&LoopInfo<L>, TypeAnalysisError> {
        self.ordered_local_scopes()
            // Fetch the closest binding
            .find_map(|scope| scope.loops.get(&name.symbol()))
            .ok_or_else(|| TypeAnalysisError::UnknownLoop(name.value().to_owned()))
    }
}

impl<L, F, P> std::fmt::Debug for ScopeStack<'_, L, F, P>
where
    L: std::fmt::Display,
    F: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope_count = self.scopes.len() + 1;
        self.ordered_scopes().enumerate().try_for_each(|(n, s)| {
            let n = scope_count - n;
            write!(f, "Scope #{n} ")?;
            std::fmt::Debug::fmt(s, f)
        })
    }
}

/// Entry in the scope stack.
pub struct Scope<'ast, L, F, P> {
    /// Binds names to predicates.
    predicates: HashMap<Symbol<'ast>, F>,
    /// Binds local names to SSA values.
    locals: HashMap<Symbol<'ast>, L>,
    /// Maps argument numbers to local symbols at this scope level.
    params: HashMap<usize, Symbol<'ast>>,
    /// Maps output numbers to local symbols at this scope level.
    outputs: HashMap<usize, Symbol<'ast>>,
    /// Maps names to loops defined on the circuit.
    ///
    /// Accessors to this table are only available if `P == ()`.
    loops: HashMap<Symbol<'ast>, LoopInfo<L>>,
    /// Indicates whether this scope entry limits the access to the locals defined in outer scopes.
    local_limit: bool,
    /// Additional payload used by the scope.
    payload: P,
}

impl<'ast, L, F, P> Scope<'ast, L, F, P> {
    /// Creates a new scope.
    fn new(local_limit: bool, payload: P) -> Self {
        Self {
            predicates: Default::default(),
            locals: Default::default(),
            params: Default::default(),
            outputs: Default::default(),
            loops: Default::default(),
            local_limit,
            payload,
        }
    }

    /// Binds the given value to a name in the predicates namespace.
    pub fn bind_predicate<M>(
        &mut self,
        name: &Identifier<'ast, M>,
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
    pub fn bind_local<M>(
        &mut self,
        name: &Identifier<'ast, M>,
        l: L,
    ) -> Result<(), TypeAnalysisError> {
        if self.locals.contains_key(&name.symbol()) {
            return Err(TypeAnalysisError::DuplicateLocal(name.value().to_owned()));
        }
        self.locals.insert(name.symbol(), l.into());
        Ok(())
    }

    /// Binds the given value to a parameter name in the locals namespace.
    pub fn bind_parameter<M>(
        &mut self,
        name: &Identifier<'ast, M>,
        l: L,
        param_no: usize,
    ) -> Result<(), TypeAnalysisError> {
        if self.locals.contains_key(&name.symbol()) || self.params.contains_key(&param_no) {
            return Err(TypeAnalysisError::DuplicateLocal(name.value().to_owned()));
        }
        self.locals.insert(name.symbol(), l.into());
        self.params.insert(param_no, name.symbol());
        Ok(())
    }

    /// Binds the given value to an output name in the locals namespace.
    pub fn bind_output<M>(
        &mut self,
        name: &Identifier<'ast, M>,
        l: L,
        output_no: usize,
    ) -> Result<(), TypeAnalysisError> {
        if self.locals.contains_key(&name.symbol()) || self.outputs.contains_key(&output_no) {
            return Err(TypeAnalysisError::DuplicateLocal(name.value().to_owned()));
        }
        self.locals.insert(name.symbol(), l.into());
        self.outputs.insert(output_no, name.symbol());
        Ok(())
    }

    pub fn payload(&self) -> &P {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut P {
        &mut self.payload
    }

    pub fn predicates(&self) -> &HashMap<Symbol<'ast>, F> {
        &self.predicates
    }

    pub fn locals(&self) -> &HashMap<Symbol<'ast>, L> {
        &self.locals
    }
}

impl<'ast, L, F> Scope<'ast, L, F, ()> {
    /// Binds the given loop information to a loop name.
    pub fn bind_loop(
        &mut self,
        name: &Identifier<'ast>,
        info: LoopInfo<L>,
    ) -> Result<(), TypeAnalysisError> {
        if self.loops.contains_key(&name.symbol()) {
            return Err(TypeAnalysisError::DuplicateLoop(name.value().to_owned()));
        }
        self.loops.insert(name.symbol(), info);
        Ok(())
    }
}

impl<L, F, P> std::fmt::Debug for Scope<'_, L, F, P>
where
    L: std::fmt::Display,
    F: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        if !self.predicates.is_empty() {
            writeln!(f, "  Predicates:")?;
            self.predicates.iter().try_for_each(|(symbol, binding)| {
                writeln!(f, "    {}: {binding}", symbol.value())
            })?;
        }
        if !self.locals.is_empty() {
            writeln!(f, "  Locals:")?;
            self.locals.iter().try_for_each(|(symbol, binding)| {
                writeln!(f, "    {}: {binding}", symbol.value())
            })?;
        }
        if !self.loops.is_empty() {
            writeln!(f, "  Loops:")?;
            self.locals.iter().try_for_each(|(symbol, binding)| {
                writeln!(f, "    {}: {binding}", symbol.value())
            })?;
        }
        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{AstContext, Span};

    use super::*;

    type Scopes<'ast> = ScopeStack<'ast, usize, usize, ()>;

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
        let mut stack = Scopes::new(());

        // Test that 'x' can be accessed but 'y' cannot
        // when 'x' is within the limit and 'y' is outside.
        stack.top().bind_local(&y, 1).unwrap();
        stack.push_local_limit(());
        stack.push(());
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
        let mut stack = Scopes::new(());

        // Test that 'x' can be accessed but 'y' cannot
        // when 'x' is right at the limit and 'y' is outside.
        stack.top().bind_local(&y, 1).unwrap();
        stack.push_local_limit(());
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
        let mut stack = Scopes::new(());

        // Test that when accesing 'x' we get the closest result back.
        stack.top().bind_local(&x, 1).unwrap();
        stack.push(());
        stack.top().bind_local(&x, 2).unwrap();

        assert_eq!(stack.find_local(&x), Ok(&2));
    }
}
