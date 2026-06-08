use std::collections::HashMap;

use crate::{
    ast::{AstContext, Symbol},
    type_analysis::{
        ArrayTypeProperties, FnTypeProperties, StructTypeProperties as _, TypeProperties,
        TypeSystem, error::TypeAnalysisError, loops::LoopInfo, scope::ScopeStack,
    },
};

/// Convenience alias for the substitutions mapping between type variable ids and resolved types.
pub(super) type Subst<T> =
    HashMap<<<T as TypeSystem>::Type as TypeProperties>::VarId, <T as TypeSystem>::Type>;

/// Shared type inference context used during type-checking.
pub(super) struct TypeInferenceCtx<'ast, T: TypeSystem> {
    scope: ScopeStack<'ast, T::Type, T::FnType, LoopInfo<T::Type>>,
    constraints: Vec<Constraint<'ast, T::Type>>,
    ts: T,
    ast: &'ast AstContext,
    subst: Subst<T>,
}

impl<'ast, T: TypeSystem> TypeInferenceCtx<'ast, T> {
    /// Creates a new context using the given type system.
    pub fn new(ts: T, ast: &'ast AstContext) -> Self {
        Self {
            scope: ScopeStack::new(()),
            constraints: Default::default(),
            ts,
            ast,
            subst: Default::default(),
        }
    }

    /// Returns a mutable reference to the type system.
    pub fn ts(&mut self) -> &mut T {
        &mut self.ts
    }

    /// Returns a mutable reference to the scopes stack.
    pub fn scope(&mut self) -> &mut ScopeStack<'ast, T::Type, T::FnType, LoopInfo<T::Type>> {
        &mut self.scope
    }

    /// Enqueues a type constraint that will get solved on the next call to unify.
    pub fn add_constraint(&mut self, lhs: T::Type, rhs: T::Type) {
        // If the types are equal we don't bother adding it.
        if lhs == rhs {
            return;
        }
        self.constraints.push(Constraint::new(lhs, rhs))
    }

    /// Enqueues an array constraint that will get solved on the next call to unify.
    pub fn add_array_constraint(&mut self, arr: T::Type, elt: T::Type) {
        // If the types are equal we don't bother adding it.
        if let Some(arr) = arr.to_array_type()
            && arr.inner_type() == elt
        {
            return;
        }

        self.constraints.push(Constraint::new_array(arr, elt))
    }

    /// Enqueues a member constraint that will get solved on the next call to unify.
    pub fn add_member_constraint(&mut self, str: T::Type, name: Symbol<'ast>, mem: T::Type) {
        self.constraints
            .push(Constraint::new_member(str, name, mem))
    }

    /// Resolves the given type to its most concrete representation.
    pub fn resolve(&mut self, t: T::Type) -> T::Type {
        Self::apply(&t, &self.subst, &mut self.ts)
    }

    /// Solves the enqueued constraints.
    pub fn unify(&mut self) -> Result<(), Vec<TypeAnalysisError>> {
        let mut errs = vec![];

        for c in std::mem::take(&mut self.constraints) {
            match c {
                Constraint::Eq { lhs, rhs } => self.unify_pair(&lhs, &rhs, &mut errs),
                Constraint::Array { arr, elt } => self.unify_array_pair(&arr, &elt, &mut errs),
                Constraint::Member { str, name, mem } => {
                    self.unify_member_pair(&str, name, &mem, &mut errs)
                }
            };
        }

        self.scope.propagate(&self.subst, &mut self.ts);
        if !errs.is_empty() {
            return Err(errs);
        }
        Ok(())
    }

    /// Handles unification of a single constraint.
    fn unify_pair(&mut self, lhs: &T::Type, rhs: &T::Type, errs: &mut Vec<TypeAnalysisError>) {
        if lhs.can_resolve_unification(&rhs) {
            return;
        }
        let lhs = Self::apply(lhs, &self.subst, &mut self.ts);
        let rhs = Self::apply(rhs, &self.subst, &mut self.ts);

        // Check for unifiablity again after application
        if lhs.can_resolve_unification(&rhs) {
            return;
        }

        if let Some(id) = lhs.var_id() {
            self.unify_type_var(id, rhs, errs);
            return;
        }

        if let Some(id) = rhs.var_id() {
            self.unify_type_var(id, lhs, errs);
            return;
        }

        if lhs.is_func_type() && rhs.is_func_type() {
            self.unify_fn_pair(
                lhs.to_func_type().unwrap(),
                rhs.to_func_type().unwrap(),
                errs,
            );
            return;
        }

        errs.push(TypeAnalysisError::UnexpectedTypes(
            lhs.to_string(),
            rhs.to_string(),
        ))
    }

    /// Handles unification of an array type constraint.
    fn unify_array_pair(
        &mut self,
        arr: &T::Type,
        elt: &T::Type,
        errs: &mut Vec<TypeAnalysisError>,
    ) {
        let arr = Self::apply(arr, &self.subst, &mut self.ts);
        let Some(arr) = arr.to_array_type() else {
            errs.push(TypeAnalysisError::ExpectedArray(arr.to_string()));
            return;
        };
        self.unify_pair(&arr.inner_type(), elt, errs)
    }

    /// Handles unification of a member type constraint.
    fn unify_member_pair(
        &mut self,
        str: &T::Type,
        name: Symbol<'ast>,
        mem: &T::Type,
        errs: &mut Vec<TypeAnalysisError>,
    ) {
        let str = Self::apply(str, &self.subst, &mut self.ts);
        let Some(str) = str.to_struct_like_type() else {
            errs.push(TypeAnalysisError::ExpectedStruct(str.to_string()));
            return;
        };

        let Some(actual) = str.get_member(name.value()) else {
            errs.push(TypeAnalysisError::ExpectedMember(
                str.to_string(),
                name.to_string(),
            ));
            return;
        };
        self.unify_pair(&actual, mem, errs);
    }

    /// Handles unification of a constraint between function types.
    fn unify_fn_pair(&mut self, lhs: T::FnType, rhs: T::FnType, errs: &mut Vec<TypeAnalysisError>) {
        if lhs.inputs().len() != rhs.inputs().len() || lhs.outputs().len() != rhs.outputs().len() {
            errs.push(TypeAnalysisError::UnexpectedTypes(
                lhs.to_string(),
                rhs.to_string(),
            ));
        }

        for (lhs, rhs) in std::iter::zip(lhs.inputs(), rhs.inputs()) {
            self.unify_pair(&lhs, &rhs, errs);
        }

        for (lhs, rhs) in std::iter::zip(lhs.outputs(), rhs.outputs()) {
            self.unify_pair(&lhs, &rhs, errs);
        }
    }

    /// Handles unification between a type variable and another type.
    fn unify_type_var(
        &mut self,
        id: <T::Type as TypeProperties>::VarId,
        t: T::Type,
        errs: &mut Vec<TypeAnalysisError>,
    ) {
        if Self::occurs(id, &t, &self.subst, &mut self.ts) {
            errs.push(TypeAnalysisError::InfiniteType(t.to_string()));
        } else {
            self.subst.insert(id, t);
        }
    }

    /// Applies substitutions until the most concrete type available is found.
    fn apply(t: &T::Type, subst: &Subst<T>, ts: &mut T) -> T::Type {
        if let Some(var_id) = t.var_id() {
            return subst
                .get(&var_id)
                .map(|t| Self::apply(t, subst, ts))
                .unwrap_or_else(|| t.clone());
        }

        if let Some(t) = t.to_func_type() {
            let ins = Self::apply_many(t.inputs(), subst, ts);
            let outs = Self::apply_many(t.outputs(), subst, ts);
            return ts.func_type(&ins, &outs).into();
        }

        if let Some(t) = t.to_array_type() {
            return t.map_inner(Self::apply(&t.inner_type(), subst, ts)).into();
        }

        if let Some(t) = t.to_struct_like_type() {
            return t.map_members(|t| Self::apply(t, subst, ts)).into();
        }

        t.clone()
    }

    /// Applies substitutions to a sequence of types until the most concrete type available is found for each.
    fn apply_many(
        t: impl IntoIterator<Item = T::Type>,
        subst: &Subst<T>,
        ts: &mut T,
    ) -> Vec<T::Type> {
        t.into_iter().map(|t| Self::apply(&t, subst, ts)).collect()
    }

    /// Returns true if the type variable is self recursive.
    fn occurs(
        id: <T::Type as TypeProperties>::VarId,
        t: &T::Type,
        subst: &Subst<T>,
        ts: &mut T,
    ) -> bool {
        let t = Self::apply(t, subst, ts);
        if let Some(other) = t.var_id() {
            return id == other;
        }

        if let Some(t) = t.to_func_type() {
            return t
                .inputs()
                .iter()
                .chain(t.outputs().iter())
                .any(|i| Self::occurs(id, i, subst, ts));
        }

        if let Some(t) = t.to_array_type() {
            return Self::occurs(id, &t.inner_type(), subst, ts);
        }

        if let Some(t) = t.to_struct_like_type() {
            return t
                .member_types()
                .iter()
                .any(|i| Self::occurs(id, i, subst, ts));
        }

        false
    }

    /// Returns a symbol containing the given string.
    pub fn symbol(&self, value: &str) -> Symbol<'ast> {
        self.ast.symbol(value)
    }

    /// Returns a reference to the AST context.
    pub(super) fn ast(&self) -> &'ast AstContext {
        self.ast
    }
}

/// A type constraint between two types.
#[derive(PartialEq, Eq, Debug)]
enum Constraint<'ast, T> {
    /// Type equality constraint
    Eq { lhs: T, rhs: T },
    /// Constraints that `elt` must be equal to the element type of `arr` and that the latter is an
    /// array type.
    Array { arr: T, elt: T },
    /// Constraints that `str` must be a struct-like type that has a member with that name and that
    /// `mem` has the same type as the member.
    Member { str: T, name: Symbol<'ast>, mem: T },
}

impl<'ast, T> Constraint<'ast, T> {
    /// Creates a new equality constraint.
    fn new(lhs: T, rhs: T) -> Self {
        Self::Eq { lhs, rhs }
    }

    /// Creates a new array constraint.
    fn new_array(arr: T, elt: T) -> Self {
        Self::Array { arr, elt }
    }

    /// Creates a new member constraint.
    fn new_member(str: T, name: Symbol<'ast>, mem: T) -> Self {
        Self::Member { str, name, mem }
    }
}

#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;
    use crate::{
        ast::{AstContext, Identifier, Span},
        type_analysis::tests::*,
    };

    type Ctx<'ast> = TypeInferenceCtx<'ast, MockTypeSystem>;

    fn ctx<'ast>(ast: &'ast AstContext) -> Ctx<'ast> {
        TypeInferenceCtx {
            ast,
            scope: ScopeStack::new(()),
            constraints: vec![],
            ts: MockTypeSystem::default(),
            subst: Default::default(),
        }
    }

    #[test]
    fn add_constraint() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().bool_type();
        let rhs = ctx.ts().fresh_var();
        ctx.add_constraint(lhs.clone(), rhs.clone());
        assert_eq!(ctx.constraints, vec![Constraint::new(lhs, rhs)]);
    }

    #[test]
    fn unify_1() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().bool_type();
        let rhs = ctx.ts().bool_type();
        ctx.add_constraint(lhs, rhs);
        ctx.unify().unwrap();
    }

    #[test]
    fn unify_2() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().felt_type();
        let rhs = ctx.ts().felt_type();
        ctx.add_constraint(lhs, rhs);
        ctx.unify().unwrap();
    }

    #[test]
    fn unify_3() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let rhs = ctx.ts().bool_type();
        ctx.add_constraint(lhs, rhs);
        ctx.unify().unwrap();
    }

    #[test]
    fn unify_4() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let var = ctx.ts().fresh_var();
        let b = ctx.ts().bool_type();
        let lhs = ctx.ts().func_type(&[var], slice::from_ref(&b));
        let rhs = ctx.ts().func_type(slice::from_ref(&b.clone()), &[b]);
        ctx.add_constraint(lhs.into(), rhs.into());
        ctx.unify().unwrap();
    }

    #[test]
    fn unify_5() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let rhs = MockArrayType::new(ctx.ts().bool_type());
        ctx.add_array_constraint(rhs.into(), lhs);
        ctx.unify().unwrap();
    }

    #[test]
    fn unify_6() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let xxx = ctx.ts().fresh_var();
        let rhs = MockArrayType::new(ctx.ts().bool_type());
        // Transitive constraint that lhs is the array element of rhs.
        //   xxx ~~ rhs
        //   xxx ~array of~ lhs
        ctx.add_constraint(xxx.clone(), rhs.into());
        ctx.add_array_constraint(xxx, lhs);
        ctx.unify().unwrap();
    }

    #[test]
    #[should_panic]
    fn unify_fail_1() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().felt_type();
        let rhs = ctx.ts().bool_type();
        ctx.add_constraint(lhs.clone(), rhs.clone());
        ctx.unify().unwrap();
    }

    #[test]
    #[should_panic]
    fn unify_fail_2() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let b = ctx.ts().bool_type();
        let rhs = ctx.ts().func_type(slice::from_ref(&lhs), &[b]);
        ctx.add_constraint(lhs, rhs.into());
        ctx.unify().unwrap();
    }

    #[test]
    #[should_panic]
    fn unify_fail_3() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let rhs = ctx.ts().fresh_var();
        let b = ctx.ts().bool_type();
        let lhs = ctx.ts().func_type(slice::from_ref(&rhs), &[b]);
        ctx.add_constraint(lhs.into(), rhs);
        ctx.unify().unwrap();
    }

    #[test]
    #[should_panic(expected = "expected type 'Bool' but got 'Felt'")]
    fn unify_fail_4() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let rhs1 = ctx.ts().bool_type();
        let rhs2 = ctx.ts().felt_type();
        ctx.add_constraint(lhs.clone(), rhs1);
        ctx.unify().expect("this one should not fail");
        ctx.add_constraint(lhs, rhs2);
        let _ = ctx.unify().map_err(|errs| {
            // We expect only one error from this check.
            assert_eq!(errs.len(), 1);
            panic!("{}", errs[0]);
        });
    }

    #[test]
    #[should_panic]
    fn unify_fail_5() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().felt_type();
        let rhs = MockArrayType::new(ctx.ts().bool_type());
        ctx.add_array_constraint(rhs.into(), lhs);
        ctx.unify().unwrap();
    }

    #[test]
    fn resolve_1() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let rhs = ctx.ts().bool_type();
        ctx.add_constraint(lhs.clone(), rhs.clone());
        ctx.unify().unwrap();
        assert_eq!(ctx.resolve(lhs), rhs);
    }

    #[test]
    fn resolve_2() {
        let ast = AstContext::new();
        let mut ctx = ctx(&ast);
        let lhs = ctx.ts().fresh_var();
        let xxx = ctx.ts().fresh_var();
        let rhs = ctx.ts().bool_type();
        ctx.add_constraint(lhs.clone(), xxx.clone());
        ctx.add_constraint(xxx.clone(), rhs.clone());
        ctx.unify().unwrap();
        assert_eq!(ctx.resolve(lhs), rhs);
    }

    #[test]
    fn propagate_1() {
        let ast_ctx = AstContext::new();
        let name = Identifier::new(ast_ctx.symbol("x"), Span::default());
        let mut ctx = ctx(&ast_ctx);

        let lhs = ctx.ts().fresh_var();
        let rhs = ctx.ts().bool_type();

        ctx.scope().push(()); // Push because the root scope does not allow locals.
        ctx.scope().top().bind_local(&name, lhs.clone()).unwrap();
        // Before unification the local binds to the var type.
        assert_eq!(*ctx.scope().find_local(&name).unwrap(), lhs);

        ctx.add_constraint(lhs.clone(), rhs.clone());
        ctx.unify().unwrap();

        // After unification we should have propagated the substitution to the binding.
        assert_eq!(*ctx.scope().find_local(&name).unwrap(), rhs);
        assert_eq!(ctx.resolve(lhs), rhs);
    }

    #[test]
    fn propagate_2() {
        let ast_ctx = AstContext::new();
        let name = Identifier::new(ast_ctx.symbol("x"), Span::default());
        let mut ctx = ctx(&ast_ctx);

        let lhs = ctx.ts().fresh_var();
        let other = ctx.ts().fresh_var();
        let rhs = ctx.ts().bool_type();

        ctx.scope().push(()); // Push because the root scope does not allow locals.
        ctx.scope().top().bind_local(&name, other.clone()).unwrap();
        // Before unification the local binds to the var type.
        assert_eq!(*ctx.scope().find_local(&name).unwrap(), other);

        ctx.add_constraint(lhs.clone(), rhs.clone());
        ctx.unify().unwrap();

        // After unification, we haven't seen the other var yet so we don't replace it.
        assert_eq!(*ctx.scope().find_local(&name).unwrap(), other);
        assert_eq!(ctx.resolve(lhs), rhs);
    }
}
