use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    ast::{ContractDecl, Identifier, Spanned, Visitable, Visitor},
    type_analysis::{
        CircuitInfo, ContractTargetInfo, TypeInferenceCtx, TypeSystem, TypingResult,
        base::BaseTypeChecker,
        block::{BlockTypeChecker, BlockTypeCheckerCfg},
        helpers::Diagnostics,
    },
};

/// Handles type checking contract declarations.
pub(super) struct ContractTypeChecker<'ctx, 'ast, 'info, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    base: BaseTypeChecker<'ctx, 'ast, T>,
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
            base: BaseTypeChecker::new(ctx, source_name),
            info,
            _marker: PhantomData,
        }
    }

    /// Configuration for type-checking blocks inside a contract.
    fn block_cfg() -> BlockTypeCheckerCfg {
        BlockTypeCheckerCfg {
            allows_invariants: true,
            allows_scoped: true,
            allows_ensure_and_require: true,
            allows_return: false,
            allows_invariant_stmts: false,
            allows_arg: true,
        }
    }

    /// Create an identifier located on the contract's span.
    ///
    /// Use it for implicit bindings like struct members.
    fn ident(&self, name: &str, decl: &ContractDecl<'ast>) -> Identifier<'ast> {
        Identifier::new(self.ctx.symbol(name), decl.span())
    }

    /// Pushes a new scope and binds the implicit environment derived from the object associated to
    /// the contract.
    ///
    /// Returns the input types of the contract's function type.
    fn push_and_bind(
        &mut self,
        decl: &ContractDecl<'ast>,
        info: impl ContractTargetInfo<'c, TypeSystem = T>,
        diags: &mut Diagnostics,
    ) -> Vec<T::Type> {
        let mut inputs = Vec::from_iter(info.self_type());

        self.ctx.scope().push_local_limit(());

        // Fill the scope with template parameters
        for info in info.template_params() {
            let name = self.ident(info.name, decl);
            let ty = info.r#type.unwrap_or_else(|| self.ctx.ts().felt_type());
            diags.extract_type_result(self.ctx.scope().top().bind_local(&name, ty), || {
                format!("while binding template parameter '{}'", info.name)
            });
        }

        // Fill the scope with input arguments as parameters (with the param number)
        for (n, t) in info.inputs().enumerate() {
            inputs.push(t.r#type.clone());
            let positional_name = self.ident(&format!("$arg[{n}]"), decl);
            diags.extract_type_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_parameter(&positional_name, t.r#type.clone(), n),
                || format!("while binding input #{n}"),
            );
            if let Some(name) = t.name {
                let name = self.ident(name, decl);
                diags.extract_type_result(
                    self.ctx.scope().top().bind_local(&name, t.r#type),
                    || format!("while binding named input '{}'", name.value()),
                );
            }
        }
        // Fill the scope with outputs in declaration order. These must be done after the inputs s.t. they
        // are in the correct order in the `inputs` vector.
        for (n, t) in info.outputs().enumerate() {
            inputs.push(t.r#type.clone());
            let positional_name = self.ident(&format!("$res[{n}]"), decl);
            diags.extract_type_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_output(&positional_name, t.r#type.clone(), n),
                || format!("while binding output #{n}"),
            );
            if let Some(name) = t.name {
                let name = self.ident(name, decl);
                diags.extract_type_result(
                    self.ctx.scope().top().bind_local(&name, t.r#type),
                    || format!("while binding named output '{}'", name.value()),
                );
            }
        }
        // Fill the scope with the struct members.
        for member in info.members() {
            let name = self.ident(member.name, decl);
            diags.extract_type_result(
                self.ctx.scope().top().bind_local(&name, member.r#type),
                || format!("while binding struct member '{}'", member.name),
            );
        }
        // Fill the scope with loop information.
        for loop_info in info.loops(self.ctx.ts()) {
            let name = Identifier::new(
                loop_info.symbolize_label(self.ctx.ast()),
                Default::default(),
            );
            diags.extract_type_result(self.ctx.scope().top().bind_loop(&name, loop_info), || {
                format!("while binding loop '{}'", name.value())
            });
        }

        inputs
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
        let mut diags = Diagnostics::new(self.source_name, decl);

        let target_info = diags
            .from_other_result(self.info.find_contract_target(decl.target()), |err| {
                format!("in contract declaration: {err}")
            })?;
        let inputs = self.push_and_bind(decl, target_info, &mut diags);
        let mut block_tc = BlockTypeChecker::new(self.source_name, self.ctx, Self::block_cfg());
        let body = diags.extract_result(decl.body().accept(&mut block_tc));
        self.ctx.scope().pop();

        diags.finish(|| {
            let t = self.ctx.ts().func_type(&inputs, &[]);
            ContractDecl::new(
                decl.target().with_meta(t.into()),
                body.unwrap(),
                decl.span(),
            )
        })
    }
}

impl<'ctx, 'ast, 'info, T, C> DerefMut for ContractTypeChecker<'ctx, 'ast, '_, 'info, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'info, TypeSystem = T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<'ctx, 'ast, 'info, T, C> Deref for ContractTypeChecker<'ctx, 'ast, '_, 'info, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'info, TypeSystem = T>,
{
    type Target = BaseTypeChecker<'ctx, 'ast, T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
