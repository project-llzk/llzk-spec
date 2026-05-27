use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    ast::{ContractDecl, Identifier, Spanned, Visitable, Visitor},
    type_analysis::{
        CircuitInfo, StructInfo, TypeInferenceCtx, TypeSystem, TypingResult,
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

    /// Configuration for type-checking inside a contract block.
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
    fn push_and_bind(
        &mut self,
        decl: &ContractDecl<'ast>,
        info: impl StructInfo<'c, TypeSystem = T>,
        diags: &mut Diagnostics,
    ) {
        self.ctx.scope().push_local_limit(());

        // Fill the scope with template parameters (as normal locals?)
        for info in info.template_params().filter(|info| info.r#type.is_some()) {
            let name = self.ident(info.name, decl);
            diags.extract_type_result(
                self.ctx
                    .scope()
                    .top()
                    .bind_local(&name, info.r#type.unwrap()),
                || format!("while binding template parameter '{}'", info.name),
            );
        }

        // Fill the scope with input arguments as parameters (with the param number)
        for (n, t) in info.inputs().enumerate() {
            let name = self.ident(&format!("${n}"), decl);
            diags.extract_type_result(self.ctx.scope().top().bind_parameter(&name, t, n), || {
                format!("while binding input #{n}")
            });
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

        let struct_info = diags.from_other_result(self.info.find_struct(decl.target()), |err| {
            format!("in contract declaration: {err}")
        })?;

        self.push_and_bind(decl, struct_info, &mut diags);
        let mut block_tc = BlockTypeChecker::new(self.source_name, self.ctx, Self::block_cfg());
        let body = diags.extract_result(decl.body().accept(&mut block_tc));
        self.ctx.scope().pop();

        diags.finish(|| {
            let t = self.ctx.ts().func_type(&[], &[]);
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
