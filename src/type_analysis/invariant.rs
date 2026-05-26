use std::ops::{Deref, DerefMut};

use crate::{
    ast::{InvariantDecl, Spanned as _, Visitable as _, Visitor},
    type_analysis::{
        TypeSystem, TypingResult,
        base::BaseTypeChecker,
        block::{BlockTypeChecker, BlockTypeCheckerCfg},
        ctx::TypeInferenceCtx,
        helpers::Diagnostics,
    },
};

/// Type checker of invariant declarations.
pub(super) struct InvariantTypeChecker<'ctx, 'ast, T: TypeSystem> {
    base: BaseTypeChecker<'ctx, 'ast, T>,
}

impl<'ctx, 'ast, T: TypeSystem> InvariantTypeChecker<'ctx, 'ast, T> {
    /// Creates a new invariant type checker.
    pub fn new(source_name: &'ast str, ctx: &'ctx mut TypeInferenceCtx<'ast, T>) -> Self {
        Self {
            base: BaseTypeChecker::new(ctx, source_name),
        }
    }

    /// Configuration for type-checking inside an invariant block.
    fn block_cfg() -> BlockTypeCheckerCfg {
        BlockTypeCheckerCfg {
            allows_invariants: false,
            allows_scoped: true, // ?
            allows_ensure_and_require: true,
            allows_return: false,
            allows_invariant_stmts: true,
        }
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<InvariantDecl<'ast>>
    for InvariantTypeChecker<'ctx, 'ast, T>
{
    type Output = TypingResult<InvariantDecl<'ast, T::Type>>;

    fn visit(&mut self, decl: &InvariantDecl<'ast>) -> Self::Output {
        // Basic invariant type-checking without checking if the loop exists.
        // That part can be added with the info traits used in other parts of the type-checker.
        // Or by another bindings table in the scope just for loops.

        let felt_type = self.ctx.ts().felt_type();
        let mut diags = Diagnostics::new(self.source_name, decl);

        self.ctx.scope().push(());

        for name in decl.bindings() {
            diags.extract_type_result(
                self.ctx.scope().top().bind_local(name, felt_type.clone()),
                || "while binding invariant identifier",
            );
        }

        let mut block_tc = BlockTypeChecker::new(self.source_name, self.ctx, Self::block_cfg());
        let body = diags.extract_result(decl.body().accept(&mut block_tc));
        self.ctx.scope().pop();

        diags.finish(|| {
            let placeholder_type = self.ctx.ts().bool_type();
            InvariantDecl::new(
                decl.loop_name().with_meta(placeholder_type),
                // All bindings are assumed to be Felts.
                decl.bindings()
                    .iter()
                    .map(|i| i.with_meta(felt_type.clone())),
                body.unwrap(),
                decl.span(),
            )
        })
    }
}

impl<'ctx, 'ast, T: TypeSystem> DerefMut for InvariantTypeChecker<'ctx, 'ast, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<'ctx, 'ast, T: TypeSystem> Deref for InvariantTypeChecker<'ctx, 'ast, T> {
    type Target = BaseTypeChecker<'ctx, 'ast, T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
