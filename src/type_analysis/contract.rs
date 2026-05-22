use crate::{
    ast::{ContractDecl, Visitor},
    type_analysis::{TypeInferenceCtx, TypeSystem, TypingResult},
};

pub(super) struct ContractTypeChecker<'ctx, 'ast, T: TypeSystem> {
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
}

impl<'ctx, 'ast, T: TypeSystem> ContractTypeChecker<'ctx, 'ast, T> {
    pub fn new(ctx: &'ctx mut TypeInferenceCtx<'ast, T>, source_name: &'ast str) -> Self {
        Self { ctx, source_name }
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<ContractDecl<'ast>> for ContractTypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<ContractDecl<'ast, T::Type>>;

    fn visit(&mut self, _: &ContractDecl<'ast>) -> Self::Output {
        let _ = self.ctx;
        let _ = self.source_name;
        todo!("contract type checking is not implemented yet")
    }
}
