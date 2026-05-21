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

impl<'ast, L, F> ScopeStack<'ast, L, F>
where
    L: std::fmt::Display,
    F: std::fmt::Display,
{
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
                for (n, i) in ins.iter_mut().enumerate() {
                    let _ = propagate_type::<T, L>(i, subst);
                }
                for (n, o) in outs.iter_mut().enumerate() {
                    let _ = propagate_type::<T, L>(o, subst);
                }
                let r = ts.func_type(&ins, &outs);
                *f = r;
            });

        fn propagate_type<T, L>(l: &mut L, subst: &Subst<T>) -> Option<()>
        where
            T: TypeSystem<Type = L>,
            L: TypeProperties + Clone + std::fmt::Display,
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

    /// Dumps the scope stack to stderr.
    pub(super) fn dump(&self) {
        let scope_count = self.scopes.len() + 1;
        self.ordered_scopes().enumerate().for_each(|(n, s)| {
            let n = scope_count - n;
            eprintln!("Scope #{n}");
            s.dump();
        });
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

impl<'ast, L, F> Scope<'ast, L, F>
where
    L: std::fmt::Display,
    F: std::fmt::Display,
{
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

    /// Dumps the scope to stderr.
    fn dump(&self) {
        eprintln!("  Predicates:");
        self.predicates.iter().for_each(|(symbol, binding)| {
            eprintln!("    {}: {binding}", symbol.value());
        });
        eprintln!("  Locals:");
        self.locals.iter().for_each(|(symbol, binding)| {
            eprintln!("    {}: {binding}", symbol.value());
        });
    }
}
