//! Test helpers and high level tests.

use super::*;

#[derive(Default)]
pub struct MockTypeSystem {
    next_var: usize,
}

impl TypeSystem for MockTypeSystem {
    type Type = MockType;
    type FnType = MockFnType;
    type ArrayType = MockArrayType;

    fn bool_type(&mut self) -> Self::Type {
        MockType::Bool
    }

    fn felt_type(&mut self) -> Self::Type {
        MockType::Felt
    }

    fn func_type(&mut self, ins: &[Self::Type], outs: &[Self::Type]) -> Self::FnType {
        MockFnType {
            ins: ins.to_vec(),
            outs: outs.to_vec(),
        }
    }

    fn fresh_var(&mut self) -> Self::Type {
        let id = self.next_var;
        self.next_var += 1;
        MockType::Var(id)
    }
}

pub type TypeVarId = usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockType {
    Felt,
    Bool,
    Fun(MockFnType),
    Array(MockArrayType),
    Var(TypeVarId),
}

impl From<MockFnType> for MockType {
    fn from(value: MockFnType) -> Self {
        Self::Fun(value)
    }
}

impl From<MockArrayType> for MockType {
    fn from(value: MockArrayType) -> Self {
        Self::Array(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockFnType {
    ins: Vec<MockType>,
    outs: Vec<MockType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockArrayType {
    inner: Box<MockType>,
}

impl std::fmt::Display for MockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockType::Felt => write!(f, "Felt"),
            MockType::Bool => write!(f, "Bool"),
            MockType::Fun(fun) => write!(f, "{fun}"),
            MockType::Var(id) => write!(f, "τ{id}"),
            MockType::Array(a) => write!(f, "{a}"),
        }
    }
}

impl std::fmt::Display for MockArrayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Array {}", &self.inner)
    }
}

impl std::fmt::Display for MockFnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fn(")?;
        self.ins[0..(self.ins.len() - 2).min(0)]
            .iter()
            .try_for_each(|i| write!(f, "{i}, "))?;
        if let Some(r#in) = self.ins.last() {
            write!(f, "{}", r#in)?;
        }
        write!(f, ") -> ")?;
        if self.outs.len() > 1 {
            write!(f, "(")?;
        }
        self.outs[0..(self.outs.len() - 2).min(0)]
            .iter()
            .try_for_each(|o| write!(f, "{o}, "))?;
        if let Some(out) = self.outs.last() {
            write!(f, "{out}")?;
        }
        if self.outs.len() > 1 {
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl FnTypeProperties for MockFnType {
    type Type = MockType;

    fn inputs(&self) -> Vec<MockType> {
        self.ins.clone()
    }

    fn outputs(&self) -> Vec<MockType> {
        self.outs.clone()
    }

    fn contains_type_vars(&self) -> bool {
        self.ins.iter().any(|i| i.contains_type_vars())
            || self.outs.iter().any(|o| o.contains_type_vars())
    }
}

impl TypeProperties for MockType {
    type FnType = MockFnType;
    type ArrayType = MockArrayType;

    type VarId = TypeVarId;

    fn is_var_type(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    fn var_id(&self) -> Option<Self::VarId> {
        match self {
            MockType::Var(id) => Some(*id),
            _ => None,
        }
    }

    fn is_func_type(&self) -> bool {
        matches!(self, Self::Fun(_))
    }

    fn to_func_type(&self) -> Option<Self::FnType> {
        match self {
            MockType::Fun(fn_type) => Some(fn_type.clone()),
            _ => None,
        }
    }

    fn is_array_type(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    fn to_array_type(&self) -> Option<Self::ArrayType> {
        match self {
            MockType::Array(a) => Some(a.clone()),
            _ => None,
        }
    }

    fn can_resolve_unification(&self, other: &Self) -> bool {
        // The mock type does not have coercible types so they resolve via equality.
        self == other
    }
}

impl ArrayTypeProperties for MockArrayType {
    type Type = MockType;

    fn inner_type(&self) -> Self::Type {
        self.inner.as_ref().clone()
    }

    fn contains_type_vars(&self) -> bool {
        self.inner.contains_type_vars()
    }
}
