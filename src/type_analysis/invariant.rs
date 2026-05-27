use std::ops::{Deref, DerefMut};

use crate::{
    ast::{Identifier, InvariantDecl, Spanned as _, Visitable as _, Visitor},
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
            allows_arg: true,
        }
    }

    fn push_and_bind(
        &mut self,
        decl: &InvariantDecl<'ast>,
        diags: &mut Diagnostics,
    ) -> TypingResult<Vec<Identifier<'ast, T::Type>>> {
        let info = diags.to_typing_result(self.ctx.scope().find_loop(decl.loop_name()), || {
            format!(
                "on invariant declaration of loop '{}'",
                decl.loop_name().value()
            )
        })?;

        diags.add_unless(decl.bindings().len() == info.bindings().len(), || {
            format!(
                "invariant declaration was expecting {} parameters but the loop has {}",
                decl.bindings().len(),
                info.bindings().len()
            )
        });

        let bindings = info
            .bindings()
            .iter()
            .map(|b| b.r#type().clone())
            .collect::<Vec<_>>();

        self.ctx.scope().push(());

        Ok(std::iter::zip(decl.bindings(), bindings)
            .map(|(name, binding)| {
                diags.extract_type_result(
                    self.ctx.scope().top().bind_local(name, binding.clone()),
                    || format!("while binding invariant identifier '{}", name.value()),
                );

                name.with_meta(binding)
            })
            .collect())
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<InvariantDecl<'ast>>
    for InvariantTypeChecker<'ctx, 'ast, T>
{
    type Output = TypingResult<InvariantDecl<'ast, T::Type>>;

    fn visit(&mut self, decl: &InvariantDecl<'ast>) -> Self::Output {
        let mut diags = Diagnostics::new(self.source_name, decl);

        let bindings = self.push_and_bind(decl, &mut diags)?;
        let mut block_tc = BlockTypeChecker::new(self.source_name, self.ctx, Self::block_cfg());
        let body = diags.extract_result(decl.body().accept(&mut block_tc));
        self.ctx.scope().pop();
        diags.finish(|| {
            let bindings_types = bindings
                .iter()
                .map(|i| i.meta().clone())
                .collect::<Vec<_>>();
            let t = self.ctx.ts().func_type(&bindings_types, &[]);
            InvariantDecl::new(
                decl.loop_name().with_meta(t.into()),
                bindings,
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
