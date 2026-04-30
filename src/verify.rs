//! Verification methods to ensure the spec lines up with the contents of the
//! LLZK IR file.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::ir::{IrMetadata, LoopMetadata, LoopScope};
use std::collections::HashSet;

/// One lexical scope frame for names introduced while verifying a block.
#[derive(Debug, Default, Clone)]
struct ScopeFrame {
    values: HashSet<String>,
    predicates: HashSet<String>,
}

/// Semantic context flags for statement and expression verification.
#[derive(Debug, Clone, Default)]
struct VerifyContext {
    in_predicate: bool,
    in_invariant: bool,
    in_step: bool,
    loop_scope: Option<LoopScope>,
}

/// Verifies the parsed document against the LLZK IR metadata.
pub fn verify_document(
    document: &Document,
    ir: &IrMetadata,
    source_name: &str,
) -> Result<(), Vec<Diagnostic>> {
    let mut verifier = Verifier {
        source_name,
        ir,
        diagnostics: Vec::new(),
        global_predicates: HashSet::new(),
    };

    verifier.collect_global_predicates(document);

    for item in &document.items {
        verifier.verify_item(item);
    }

    if verifier.diagnostics.is_empty() {
        Ok(())
    } else {
        Err(verifier.diagnostics)
    }
}

/// Semantic verifier for a single parsed document.
struct Verifier<'a> {
    /// Source path/name attached to emitted diagnostics.
    source_name: &'a str,
    /// IR metadata used for symbol and loop-label resolution.
    ir: &'a IrMetadata,
    /// Semantic diagnostics accumulated during this verification run.
    diagnostics: Vec<Diagnostic>,
    /// Top-level predicate names pre-collected for duplicate detection and visibility.
    global_predicates: HashSet<String>,
}

impl<'a> Verifier<'a> {
    /// Collects top-level predicate names before verifying bodies.
    fn collect_global_predicates(&mut self, document: &Document) {
        for item in &document.items {
            if let Item::Predicate(predicate) = item
                && !self.global_predicates.insert(predicate.name.name.clone())
            {
                self.push(
                    format!("duplicate predicate `{}`", predicate.name.name),
                    Some(predicate.name.span),
                );
            }
        }
    }

    /// Verifies one top-level item in the document.
    fn verify_item(&mut self, item: &Item) {
        match item {
            Item::Contract(contract) => self.verify_contract(contract),
            Item::Predicate(predicate) => self.verify_predicate(predicate, &[]),
        }
    }

    /// Verifies a contract body against IR-visible names and symbols.
    fn verify_contract(&mut self, contract: &ContractDecl) {
        if !self.ir.defined_symbols.contains(&contract.target.name) {
            self.push(
                format!("unknown contract target `{}`", contract.target.name),
                Some(contract.target.span),
            );
        }

        let mut scopes = vec![ScopeFrame::default()];
        let context = VerifyContext {
            loop_scope: self.contract_loop_scope(&contract.target.name),
            ..VerifyContext::default()
        };
        self.verify_block(&contract.body, &mut scopes, &context);
    }

    /// Verifies a predicate declaration with inherited outer lexical scopes.
    fn verify_predicate(&mut self, predicate: &PredicateDecl, inherited: &[ScopeFrame]) {
        let mut scopes = inherited.to_vec();
        scopes.push(ScopeFrame::default());
        for param in &predicate.params {
            self.define_value(&mut scopes, param, "duplicate local binding");
        }

        match &predicate.body {
            PredicateBody::Block(block) => {
                let context = VerifyContext {
                    in_predicate: true,
                    ..VerifyContext::default()
                };
                self.verify_block(block, &mut scopes, &context)
            }
            PredicateBody::Expr(expression) => {
                let context = VerifyContext::default();
                self.verify_expression(expression, &mut scopes, &context)
            }
        }
    }

    /// Verifies a block inside a fresh lexical scope frame.
    fn verify_block(
        &mut self,
        block: &Block,
        scopes: &mut Vec<ScopeFrame>,
        context: &VerifyContext,
    ) {
        scopes.push(ScopeFrame::default());
        for statement in &block.statements {
            self.verify_statement(statement, scopes, context);
        }
        scopes.pop();
    }

    /// Verifies a single statement and updates lexical scope state as needed.
    fn verify_statement(
        &mut self,
        statement: &Statement,
        scopes: &mut Vec<ScopeFrame>,
        context: &VerifyContext,
    ) {
        match statement {
            Statement::Scoped { statement, .. } => {
                self.verify_statement(statement, scopes, context)
            }
            Statement::Block(block) => self.verify_block(block, scopes, context),
            Statement::Require { expression, .. } | Statement::Ensure { expression, .. } => {
                self.verify_expression(expression, scopes, context)
            }
            Statement::Let { name, value, .. } => {
                self.verify_expression(value, scopes, context);
                self.define_value(scopes, name, "duplicate local binding");
            }
            Statement::Unused { name, .. } => {
                if !self.name_visible(scopes, &name.name) {
                    self.push(
                        format!("unused references unknown symbol `{}`", name.name),
                        Some(name.span),
                    );
                }
            }
            Statement::Return { expression, span } => {
                if !context.in_predicate {
                    self.push("return is only valid inside predicates", Some(*span));
                }
                self.verify_expression(expression, scopes, context);
            }
            Statement::Increases { expression, span } => {
                if !context.in_invariant {
                    self.push("increases is only valid inside invariants", Some(*span));
                }
                self.verify_expression(
                    expression,
                    scopes,
                    &VerifyContext {
                        in_step: false,
                        ..context.clone()
                    },
                );
            }
            Statement::Decreases { expression, span } => {
                if !context.in_invariant {
                    self.push("decreases is only valid inside invariants", Some(*span));
                }
                self.verify_expression(
                    expression,
                    scopes,
                    &VerifyContext {
                        in_step: false,
                        ..context.clone()
                    },
                );
            }
            Statement::Step { expression, span } => {
                if !context.in_invariant {
                    self.push("step is only valid inside invariants", Some(*span));
                }
                self.verify_expression(
                    expression,
                    scopes,
                    &VerifyContext {
                        in_step: true,
                        ..context.clone()
                    },
                );
            }
            Statement::Invariant(invariant) => {
                match self.lookup_loop(&invariant.loop_name.name, context.loop_scope.as_ref()) {
                    Some(loop_metadata)
                        if loop_metadata.binding_count != invariant.bindings.len() =>
                    {
                        self.push(
                            format!(
                                "loop `{}` expects {} invariant bindings, found {}",
                                invariant.loop_name.name,
                                loop_metadata.binding_count,
                                invariant.bindings.len()
                            ),
                            Some(invariant.loop_name.span),
                        );
                    }
                    Some(_) => {}
                    None => {
                        self.push(
                            format!("unknown loop `{}`", invariant.loop_name.name),
                            Some(invariant.loop_name.span),
                        );
                    }
                }
                scopes.push(ScopeFrame::default());
                for binding in &invariant.bindings {
                    self.define_value(scopes, binding, "duplicate local binding");
                }
                for statement in &invariant.body.statements {
                    self.verify_statement(
                        statement,
                        scopes,
                        &VerifyContext {
                            in_invariant: true,
                            in_step: false,
                            ..context.clone()
                        },
                    );
                }
                scopes.pop();
            }
            Statement::Predicate(predicate) => {
                if !scopes
                    .last_mut()
                    .expect("scope")
                    .predicates
                    .insert(predicate.name.name.clone())
                {
                    self.push(
                        format!("duplicate predicate `{}`", predicate.name.name),
                        Some(predicate.name.span),
                    );
                }
                self.verify_predicate(predicate, scopes);
            }
            Statement::Empty { .. } => {}
        }
    }

    /// Verifies an expression recursively and reports unresolved names.
    fn verify_expression(
        &mut self,
        expression: &Expression,
        scopes: &mut Vec<ScopeFrame>,
        context: &VerifyContext,
    ) {
        match expression {
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.verify_expression(condition, scopes, context);
                self.verify_expression(then_branch, scopes, context);
                self.verify_expression(else_branch, scopes, context);
            }
            Expression::Binary { left, right, .. } => {
                self.verify_expression(left, scopes, context);
                self.verify_expression(right, scopes, context);
            }
            Expression::Unary { expr, .. } => self.verify_expression(expr, scopes, context),
            Expression::Index { target, index, .. } => {
                self.verify_expression(target, scopes, context);
                self.verify_expression(index, scopes, context);
            }
            Expression::Member { target, span, .. } => {
                self.verify_expression(target, scopes, context);
                if let Some(path) = self.expression_path(expression) {
                    if self.ir.all_member_paths.contains(&path) {
                        if !self.ir.accessible_member_paths.contains(&path) {
                            self.push(format!("member `{path}` is not public"), Some(*span));
                        }
                    } else {
                        self.push(format!("unknown identifier `{path}`"), Some(*span));
                    }
                }
            }
            Expression::Call { callee, args, .. } => {
                if !self.name_visible(scopes, &callee.name) {
                    self.push(
                        format!("unknown identifier `{}`", callee.name),
                        Some(callee.span),
                    );
                }
                for arg in args {
                    self.verify_expression(arg, scopes, context);
                }
            }
            Expression::Quantifier {
                binding,
                domain,
                body,
                ..
            } => {
                match domain {
                    QuantifierDomain::Range { start, end, .. } => {
                        self.verify_expression(start, scopes, context);
                        self.verify_expression(end, scopes, context);
                    }
                    QuantifierDomain::Expr(expr) => self.verify_expression(expr, scopes, context),
                }
                scopes.push(ScopeFrame::default());
                self.define_value(scopes, binding, "duplicate local binding");
                self.verify_expression(body, scopes, context);
                scopes.pop();
            }
            Expression::Len { target, .. } => self.verify_expression(target, scopes, context),
            Expression::Old { expression, span } => {
                if !context.in_step {
                    self.push("old is only valid inside step expressions", Some(*span));
                }
                self.verify_expression(expression, scopes, context);
            }
            // TODO: `arg[N]` is a temporary workaround for unnamed function arguments.
            // Once a solution for referencing arguments using source-language naming
            // is implemented in llzk-lib, we will default to resolving arguments in
            // that way and use the `arg` lookup only as a backup.
            Expression::Arg { .. } => {}
            Expression::Symbol(identifier) => {
                if !self.name_visible(scopes, &identifier.name) {
                    self.push(
                        format!("unknown identifier `{}`", identifier.name),
                        Some(identifier.span),
                    );
                }
            }
            Expression::Nondet { .. } | Expression::Boolean { .. } | Expression::Number { .. } => {}
        }
    }

    /// Defines a local value in the current (innermost) lexical scope.
    fn define_value(&mut self, scopes: &mut [ScopeFrame], identifier: &Identifier, message: &str) {
        let scope = scopes.last_mut().expect("scope frame");
        if !scope.values.insert(identifier.name.clone()) {
            self.push(
                format!("{message} `{}`", identifier.name),
                Some(identifier.span),
            );
        }
    }

    /// Returns whether a name is visible from lexical scopes, predicates, or IR.
    fn name_visible(&self, scopes: &[ScopeFrame], name: &str) -> bool {
        scopes
            .iter()
            .rev()
            .any(|scope| scope.values.contains(name) || scope.predicates.contains(name))
            || self.global_predicates.contains(name)
            || self.ir.defined_symbols.contains(name)
    }

    fn expression_path(&self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::Symbol(identifier) => Some(identifier.name.clone()),
            Expression::Member { target, member, .. } => {
                Some(format!("{}.{}", self.expression_path(target)?, member.name))
            }
            _ => None,
        }
    }

    /// Resolves loop names by owner scope.
    fn lookup_loop(&self, name: &str, scope: Option<&LoopScope>) -> Option<LoopMetadata> {
        match scope {
            Some(scope) => self
                .ir
                .labeled_loops
                .get(&(scope.clone(), name.to_string()))
                .cloned(),
            None => None,
        }
    }

    /// Maps a contract target to the generated loop scope used by unlabeled loops.
    fn contract_loop_scope(&self, target: &str) -> Option<LoopScope> {
        let struct_scope = LoopScope::Struct(target.to_string());
        if self
            .ir
            .labeled_loops
            .keys()
            .any(|(scope, _)| scope == &struct_scope)
        {
            return Some(struct_scope);
        }

        let function_scope = LoopScope::Function(target.to_string());
        if self
            .ir
            .labeled_loops
            .keys()
            .any(|(scope, _)| scope == &function_scope)
        {
            return Some(function_scope);
        }

        None
    }

    /// Records a diagnostic for the current source file.
    fn push(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.diagnostics
            .push(Diagnostic::new(self.source_name, message, span));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrMetadata, LoopKind, LoopMetadata, LoopScope};
    use crate::parser::parse_document;
    use std::collections::HashMap;

    fn ir() -> IrMetadata {
        IrMetadata {
            defined_symbols: ["Foo", "out", "in", "helper", "child", "pod"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            all_member_paths: [
                "child.pub_out".to_string(),
                "child.secret".to_string(),
                "pod.count".to_string(),
                "pod.flag".to_string(),
            ]
            .into_iter()
            .collect(),
            accessible_member_paths: [
                "child.pub_out".to_string(),
                "pod.count".to_string(),
                "pod.flag".to_string(),
            ]
            .into_iter()
            .collect(),
            labeled_loops: [(
                (LoopScope::Struct("Foo".to_string()), "loop0".to_string()),
                LoopMetadata {
                    kind: LoopKind::For,
                    binding_count: 4,
                    scope: LoopScope::Struct("Foo".to_string()),
                    explicit_label: false,
                },
            )]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        }
    }

    #[test]
    fn rejects_missing_contract_target() {
        let document = parse_document("test.spec", "contract for Missing { ensure out == 0; }")
            .expect("parse success");
        let diagnostics =
            verify_document(&document, &ir(), "test.spec").expect_err("verify failure");
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("unknown contract target"))
        );
    }

    #[test]
    fn rejects_return_outside_predicate() {
        let document =
            parse_document("test.spec", "contract for Foo { return out; }").expect("parse success");
        let diagnostics =
            verify_document(&document, &ir(), "test.spec").expect_err("verify failure");
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("return is only valid"))
        );
    }

    #[test]
    fn allows_nested_shadowing_but_rejects_same_scope_duplicates() {
        let source = r#"
contract for Foo {
  let x = out;
  compute { let x = in; }
  let x = helper;
}
"#;
        let document = parse_document("test.spec", source).expect("parse success");
        let diagnostics =
            verify_document(&document, &ir(), "test.spec").expect_err("verify failure");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("duplicate local binding"));
    }

    #[test]
    fn verifies_loop_binding_count() {
        let source =
            "contract for Foo { invariant for loop0(lb, i, ub, step) { ensure i >= lb; } }";
        let document = parse_document("test.spec", source).expect("parse success");
        verify_document(&document, &ir(), "test.spec").expect("verify success");

        let source = "contract for Foo { invariant for loop0(i) { ensure i >= 0; } }";
        let document = parse_document("test.spec", source).expect("parse success");
        let diagnostics =
            verify_document(&document, &ir(), "test.spec").expect_err("verify failure");
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("expects 4 invariant bindings"))
        );
    }

    #[test]
    fn restricts_step_and_old_to_invariant_steps() {
        let source = r#"
contract for Foo {
  invariant for loop0(lb, i, ub, step) {
    step i == old(i) + step;
  }
}
"#;
        let document = parse_document("test.spec", source).expect("parse success");
        verify_document(&document, &ir(), "test.spec").expect("verify success");

        let source = "contract for Foo { ensure old(out) == out; step out == out; }";
        let document = parse_document("test.spec", source).expect("parse success");
        let diagnostics =
            verify_document(&document, &ir(), "test.spec").expect_err("verify failure");
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("old is only valid inside step"))
        );
        assert!(diagnostics.iter().any(|diag| {
            diag.message
                .contains("step is only valid inside invariants")
        }));
    }

    #[test]
    fn validates_nested_member_access() {
        let source = "contract for Foo { ensure child.pub_out == pod.count; }";
        let document = parse_document("test.spec", source).expect("parse success");
        verify_document(&document, &ir(), "test.spec").expect("verify success");

        let source = "contract for Foo { ensure child.secret == 0; }";
        let document = parse_document("test.spec", source).expect("parse success");
        let diagnostics =
            verify_document(&document, &ir(), "test.spec").expect_err("verify failure");
        assert!(
            diagnostics
                .iter()
                .any(|diag| { diag.message.contains("member `child.secret` is not public") })
        );
    }
}
