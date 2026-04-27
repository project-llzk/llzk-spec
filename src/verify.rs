//! Verification methods to ensure the spec lines up with the contents of the
//! LLZK IR file.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::ir::IrMetadata;
use std::collections::HashSet;

/// One lexical scope frame for names introduced while verifying a block.
#[derive(Debug, Default, Clone)]
struct ScopeFrame {
    values: HashSet<String>,
    predicates: HashSet<String>,
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

/// Stateful semantic verifier for a single parsed document.
///
/// The verifier is stateful because it accumulates diagnostics and keeps track
/// of document-wide predicate visibility while walking nested lexical scopes.
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
        self.verify_block(&contract.body, &mut scopes, false);
    }

    /// Verifies a predicate declaration with inherited outer lexical scopes.
    fn verify_predicate(&mut self, predicate: &PredicateDecl, inherited: &[ScopeFrame]) {
        let mut scopes = inherited.to_vec();
        scopes.push(ScopeFrame::default());
        for param in &predicate.params {
            self.define_value(&mut scopes, param, "duplicate local binding");
        }

        match &predicate.body {
            PredicateBody::Block(block) => self.verify_block(block, &mut scopes, true),
            PredicateBody::Expr(expression) => self.verify_expression(expression, &mut scopes),
        }
    }

    /// Verifies a block inside a fresh lexical scope frame.
    fn verify_block(&mut self, block: &Block, scopes: &mut Vec<ScopeFrame>, in_predicate: bool) {
        scopes.push(ScopeFrame::default());
        for statement in &block.statements {
            self.verify_statement(statement, scopes, in_predicate);
        }
        scopes.pop();
    }

    /// Verifies a single statement and updates lexical scope state as needed.
    fn verify_statement(
        &mut self,
        statement: &Statement,
        scopes: &mut Vec<ScopeFrame>,
        in_predicate: bool,
    ) {
        match statement {
            Statement::Scoped { statement, .. } => {
                self.verify_statement(statement, scopes, in_predicate)
            }
            Statement::Block(block) => self.verify_block(block, scopes, in_predicate),
            Statement::Require { expression, .. } | Statement::Ensure { expression, .. } => {
                self.verify_expression(expression, scopes)
            }
            Statement::Let { name, value, .. } => {
                self.verify_expression(value, scopes);
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
                if !in_predicate {
                    self.push("return is only valid inside predicates", Some(*span));
                }
                self.verify_expression(expression, scopes);
            }
            Statement::Invariant(invariant) => {
                if !self.ir.loop_labels.contains(&invariant.loop_label.name) {
                    self.push(
                        format!("unknown loop label `{}`", invariant.loop_label.name),
                        Some(invariant.loop_label.span),
                    );
                }
                scopes.push(ScopeFrame::default());
                self.define_value(scopes, &invariant.induction_var, "duplicate local binding");
                for statement in &invariant.body.statements {
                    self.verify_statement(statement, scopes, in_predicate);
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
    fn verify_expression(&mut self, expression: &Expression, scopes: &mut Vec<ScopeFrame>) {
        match expression {
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.verify_expression(condition, scopes);
                self.verify_expression(then_branch, scopes);
                self.verify_expression(else_branch, scopes);
            }
            Expression::Binary { left, right, .. } => {
                self.verify_expression(left, scopes);
                self.verify_expression(right, scopes);
            }
            Expression::Unary { expr, .. } => self.verify_expression(expr, scopes),
            Expression::Index { target, index, .. } => {
                self.verify_expression(target, scopes);
                self.verify_expression(index, scopes);
            }
            Expression::Call { callee, args, .. } => {
                if !self.name_visible(scopes, &callee.name) {
                    self.push(
                        format!("unknown identifier `{}`", callee.name),
                        Some(callee.span),
                    );
                }
                for arg in args {
                    self.verify_expression(arg, scopes);
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
                        self.verify_expression(start, scopes);
                        self.verify_expression(end, scopes);
                    }
                    QuantifierDomain::Expr(expr) => self.verify_expression(expr, scopes),
                }
                scopes.push(ScopeFrame::default());
                self.define_value(scopes, binding, "duplicate local binding");
                self.verify_expression(body, scopes);
                scopes.pop();
            }
            Expression::Len { target, .. } => self.verify_expression(target, scopes),
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

    /// Defines a local value in the innermost lexical scope.
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
            || self.ir.visible_names.contains(name)
    }

    /// Records a semantic diagnostic for the current source file.
    fn push(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.diagnostics
            .push(Diagnostic::new(self.source_name, message, span));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrMetadata;
    use crate::parser::parse_document;

    fn ir() -> IrMetadata {
        IrMetadata {
            defined_symbols: ["Foo".to_string()].into_iter().collect(),
            visible_names: ["Foo", "out", "in", "helper"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            loop_labels: ["loop1".to_string()].into_iter().collect(),
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
}
