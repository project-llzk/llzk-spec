//! Test helpers and high level tests.

use super::*;

#[derive(Default)]
pub struct MockTypeSystem {
    next_var: usize,
}

impl TypeSystem for MockTypeSystem {
    type Type = MockType;
    type FnType = MockFnType;

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
    Var(TypeVarId),
}

impl From<MockFnType> for MockType {
    fn from(value: MockFnType) -> Self {
        Self::Fun(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockFnType {
    ins: Vec<MockType>,
    outs: Vec<MockType>,
}

impl std::fmt::Display for MockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockType::Felt => write!(f, "Felt"),
            MockType::Bool => write!(f, "Bool"),
            MockType::Fun(fun) => write!(f, "{fun}"),
            MockType::Var(id) => write!(f, "τ{id}"),
        }
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
}
