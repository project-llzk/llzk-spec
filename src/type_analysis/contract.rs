use std::marker::PhantomData;

use crate::{
    ast::{ContractDecl, Identifier, Spanned, Visitable, Visitor},
    diagnostic::Diagnostic,
    type_analysis::{
        CircuitInfo, StructInfo, TypeInferenceCtx, TypeSystem, TypingResult,
        block::{BlockTypeChecker, BlockTypeCheckerCfg},
        helpers::extract_result,
    },
};

/// Handles type checking contract declarations.
pub(super) struct ContractTypeChecker<'ctx, 'ast, 'info, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    source_name: &'ast str,
    ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
    info: &'info C,
    _marker: PhantomData<&'c ()>,
}

impl<'ctx, 'ast, 'info, 'c, T, C> ContractTypeChecker<'ctx, 'ast, 'info, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    /// Creates a new contract type checker.
    pub fn new(
        ctx: &'ctx mut TypeInferenceCtx<'ast, T>,
        info: &'info C,
        source_name: &'ast str,
    ) -> Self {
        Self {
            ctx,
            source_name,
            info,
            _marker: PhantomData,
        }
    }
}

impl<'ast, 'ctx, 'c, T, C> Visitor<ContractDecl<'ast>>
    for ContractTypeChecker<'ctx, 'ast, '_, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    type Output = TypingResult<ContractDecl<'ast, T::Type>>;

    fn visit(&mut self, decl: &ContractDecl<'ast>) -> Self::Output {
        let mut diags = vec![];

        let struct_info = self.info.find_struct(decl.target()).map_err(|err| {
            vec![Diagnostic::new(
                self.source_name,
                format!("in contract declaration: {err}"),
                Some(decl.span()),
            )]
        })?;

        // 1. Push a new scope with a local limit
        self.ctx.scope().push_local_limit(());
        // 2. Fill scope with template parameters (as normal locals?)
        for info in struct_info.template_params() {
            let Some(t) = info.r#type else {
                continue;
            };
            let name = Identifier::new(self.ctx.symbol(info.name), decl.span());
            extract_result(
                self.ctx.scope().top().bind_local(&name, t).map_err(|err| {
                    err.into_diags(
                        self.source_name,
                        Some(decl.span()),
                        format!("while binding template parameter '{}'", info.name),
                    )
                }),
                &mut diags,
            );
        }
        // 3. Fill scope with input arguments as parameters (with the param number)
        for (n, t) in struct_info.inputs().enumerate() {
            let name = Identifier::new(self.ctx.symbol(&format!("${n}")), decl.span());
            extract_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_parameter(&name, t, n)
                    .map_err(|err| {
                        err.into_diags(
                            self.source_name,
                            Some(decl.span()),
                            format!("while binding input #{n}"),
                        )
                    }),
                &mut diags,
            );
        }
        // 4. Fill scope with the struct members.
        for member in struct_info.members() {
            let name = Identifier::new(self.ctx.symbol(member.name), decl.span());
            extract_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_local(&name, member.r#type)
                    .map_err(|err| {
                        err.into_diags(
                            self.source_name,
                            Some(decl.span()),
                            format!("while binding struct member '{}'", member.name),
                        )
                    }),
                &mut diags,
            );
        }
        // 5. Type-check the body.
        let mut block_tc = BlockTypeChecker::new(
            self.source_name,
            self.ctx,
            BlockTypeCheckerCfg {
                allows_invariants: true,
                allows_scoped: true,
                allows_ensure_and_require: true,
                allows_return: false,
                allows_invariant_stmts: false,
            },
        );
        let body = extract_result(decl.body().accept(&mut block_tc), &mut diags);
        // 6. Pop the scope.
        self.ctx.scope().pop();

        if !diags.is_empty() {
            return Err(diags);
        }
        Ok(ContractDecl::new(
            // Bool type for now just to put something. The type of this identifier must be
            // the type used for creating the corresponding MLIR op, if it's typed. Otherwise
            // we should set it to an obvious placeholder like a void type.
            decl.target().with_meta(self.ctx.ts().bool_type()),
            body.unwrap(),
            decl.span(),
        ))
    }
}
