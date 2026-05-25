use crate::{
    ast::{Block, InvariantDecl, Spanned as _, Statement, Visitable as _, Visitor},
    diagnostic::Diagnostic,
    type_analysis::{
        TypeSystem, TypingResult,
        block::{BlockTypeChecker, BlockTypeCheckerCfg},
        ctx::TypeInferenceCtx,
        expression::{ExpressionTypeChecker, ExpressionTypeCheckerCfg},
        helpers::{check_many, extract_result},
    },
};

/// Type checker of invariant declarations.
pub(super) struct InvariantTypeChecker<'ctx, 'ast, T: TypeSystem> {
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
}

impl<'ctx, 'ast, T: TypeSystem> InvariantTypeChecker<'ctx, 'ast, T> {
    /// Creates a new invariant type checker.
    pub fn new(ctx: &'ctx mut TypeInferenceCtx<'ast, T>, source_name: &'ast str) -> Self {
        Self { ctx, source_name }
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
        let mut diags = vec![];

        self.ctx.scope().push(());

        for name in decl.bindings() {
            extract_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_local(name, felt_type.clone())
                    .map_err(|err| {
                        err.into_diags(
                            self.source_name,
                            Some(decl.span()),
                            "while binding invariant identifier",
                        )
                    }),
                &mut diags,
            );
        }

        let mut block_tc = BlockTypeChecker::new(
            self.source_name,
            self.ctx,
            BlockTypeCheckerCfg {
                allows_invariants: false,
                allows_scoped: true, // ?
                allows_ensure_and_require: true,
                allows_return: false,
                allows_invariant_stmts: true,
            },
        );
        let body = extract_result(decl.body().accept(&mut block_tc), &mut diags);
        self.ctx.scope().pop();

        if !diags.is_empty() {
            return Err(diags);
        }

        let placeholder_type = self.ctx.ts().bool_type();
        Ok(InvariantDecl::new(
            decl.loop_name().with_meta(placeholder_type),
            // All bindings are assumed to be Felts.
            decl.bindings()
                .iter()
                .map(|i| i.with_meta(felt_type.clone())),
            body.unwrap(),
            decl.span(),
        ))
    }
}
