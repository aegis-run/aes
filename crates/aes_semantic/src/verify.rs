use aes_ast::{Ast, ExprTerm};
use aes_foundation::Reporter;

use crate::{Context, errors};

pub(crate) fn verify_schema<'src, R: Reporter>(ctx: &mut Context<'src, R>, ast: &'src Ast<'src>) {
    aes_visit::schema(&mut Verifier {
        ast,
        ctx,
        scope: None,
        res_ctx: ResolutionContext::Relation,
        scratch: Vec::with_capacity(32),
    });
}

#[derive(Debug, Clone, Copy)]
enum ResolutionContext {
    Relation,
    Permission,
}

struct Verifier<'c, 'src, R: Reporter> {
    ast: &'src Ast<'src>,
    ctx: &'c mut Context<'src, R>,
    scratch: Vec<aes_ast::TypeDefId>,

    scope: Option<aes_ast::TypeDefId>,
    res_ctx: ResolutionContext,
}

impl<'c, 'src, R: Reporter> aes_visit::Visitor<'src> for Verifier<'c, 'src, R> {
    fn ast(&self) -> &'src Ast<'src> {
        self.ast
    }

    fn type_def(&mut self, id: aes_ast::TypeDefId) {
        self.scope = Some(id);
        aes_visit::walk_type_def(self, id);
        self.scope = None;
    }

    fn let_member(&mut self, id: aes_ast::LetMemberId) {
        self.res_ctx = ResolutionContext::Relation;
        aes_visit::walk_let_member(self, id);
    }

    fn def_member(&mut self, id: aes_ast::DefMemberId) {
        self.res_ctx = ResolutionContext::Permission;
        aes_visit::walk_def_member(self, id);
    }

    fn expr_type_ref(&mut self, expr: aes_ast::ExprTermTypeRef) {
        let span = expr.span;
        if !self.in_relation() {
            return self.ctx.report(errors::type_ref_in_permission(span));
        }

        let name = span.text(self.ctx.source);
        if self.ctx.index.type_(name).is_none() {
            self.ctx.report(errors::unknown_type(span))
        }
    }

    fn expr_userset_type_ref(&mut self, expr: aes_ast::ExprTermUsersetTypeRef) {
        if !self.in_relation() {
            return self.ctx.report(errors::type_ref_in_permission(expr.ty));
        }

        let ty_name = expr.ty.text(self.ctx.source);
        let Some(tid) = self.ctx.index.type_(ty_name) else {
            return self.ctx.report(errors::unknown_type(expr.ty));
        };

        let member_name = expr.member.text(self.ctx.source);
        if self.ctx.index.has_relation(tid, member_name) {
            return;
        }

        if self.ctx.index.has_permission(tid, member_name) {
            return self
                .ctx
                .report(errors::userset_ref_to_permission(expr.member, ty_name));
        }

        self.ctx
            .report(errors::unknown_member(expr.member, ty_name));
    }

    fn expr_self_ref(&mut self, expr: aes_ast::ExprTermSelfRef) {
        if !self.in_permission() {
            return self.ctx.report(errors::self_ref_in_relation(expr.span));
        }

        let name = expr.span.text(self.ctx.source);

        if !self.ctx.index.has_member(self.scope(), name) {
            self.ctx.report(errors::unknown_relation(expr.span))
        }
    }

    fn expr_traversal(&mut self, expr: aes_ast::ExprTermTraversal) {
        if !self.in_permission() {
            return self.ctx.report(errors::self_ref_in_relation(expr.relation));
        }

        let relation_name = expr.relation.text(self.ctx.source);

        let Some((_, let_expr)) = self.ctx.index.relation(self.scope(), relation_name) else {
            let diagnostic = if self.ctx.index.has_permission(self.scope(), relation_name) {
                errors::traversal_on_permission(expr.relation)
            } else {
                errors::unknown_relation(expr.relation)
            };

            return self.ctx.report(diagnostic);
        };

        let permission_name = expr.permission.text(self.ctx.source);

        self.scratch.clear();
        self.collect_type_refs(let_expr);

        for &tid in self.scratch.iter() {
            if !self.ctx.index.has_member(tid, permission_name) {
                self.ctx.report(errors::unknown_permission_on_type(
                    expr.permission,
                    permission_name,
                    self.ctx.index.type_name(tid).unwrap_or("<unknown>"),
                ));
            }
        }
    }
}

impl<'c, 'src, R: Reporter> Verifier<'c, 'src, R> {
    #[allow(clippy::unreachable)]
    fn scope(&self) -> aes_ast::TypeDefId {
        debug_assert!(
            self.scope.is_some(),
            "scope() called outside type_def traversal"
        );

        let Some(id) = self.scope else {
            unreachable!("scope() called outside type_def traversal");
        };

        id
    }

    fn in_relation(&self) -> bool {
        matches!(self.res_ctx, ResolutionContext::Relation)
    }

    fn in_permission(&self) -> bool {
        matches!(self.res_ctx, ResolutionContext::Permission)
    }

    fn collect_type_refs(&mut self, expr_id: aes_ast::ExprId) {
        match self.ast.exprs().at(expr_id).term() {
            ExprTerm::TypeRef(expr) => {
                let name = expr.span.text(self.ctx.source);
                if let Some(tid) = self.ctx.index.type_(name) {
                    self.scratch.push(tid);
                }
            }
            ExprTerm::UsersetTypeRef(expr) => {
                let name = expr.ty.text(self.ctx.source);
                if let Some(tid) = self.ctx.index.type_(name) {
                    self.scratch.push(tid);
                }
            }
            ExprTerm::Binary(expr) => {
                self.collect_type_refs(expr.lhs);
                self.collect_type_refs(expr.rhs);
            }
            ExprTerm::Paren(expr) => self.collect_type_refs(expr.inner),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use aes_allocator::Allocator;
    use aes_testing::assert_code;
    use indoc::indoc;

    use crate::declare::declare_schema;

    use super::*;

    fn run(source: &str) -> aes_testing::Reporter {
        let alloc = Allocator::new();
        let (ast, errs) = aes_parser::Parser::new(&alloc, source).parse();
        assert!(errs.is_empty());

        let mut ctx = Context::new(&alloc, source, 8, aes_testing::Reporter::default());
        declare_schema(&mut ctx, &ast);
        assert!(ctx.reporter.is_clean());

        verify_schema(&mut ctx, &ast);

        ctx.reporter
    }

    mod type_ref {
        use super::*;

        #[test]
        fn valid_type_ref_in_relation_no_errors() {
            let source = indoc! {r#"
                type group {}

                type user {
                  let owner = group;
                }
            "#};
            let reporter = run(source);
            assert!(reporter.is_clean());
        }

        #[test]
        fn unknown_type_ref_emits_error() {
            let source = indoc! {r#"
                type user {
                  let owner = ghost;
                }
            "#};
            let reporter = run(source);

            assert_code(&reporter, "aes::semantic(unknown_type)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn type_ref_in_permission_emits_error() {
            let source = indoc! {r#"
                type group {}

                type user {
                  def read = group;
                }
            "#};

            let reporter = run(source);

            assert_code(&reporter, "aes::semantic(type_ref_in_permission)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }
    }

    mod userset_type_ref {
        use super::*;

        #[test]
        fn valid_userset_ref_no_errors() {
            let source = indoc! {r#"
                type group {
                  let member = user;
                }

                type user {
                  let owner = group::member;
                }
            "#};
            let reporter = run(source);
            assert!(reporter.is_clean());
        }

        #[test]
        fn userset_ref_unknown_type_emits_error() {
            let source = indoc! {r#"
                type user {
                  let owner = ghost::member;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(unknown_type)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn userset_ref_unknown_member_emits_error() {
            let source = indoc! {r#"
                type group {}

                type user {
                  let owner = group::ghost;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(unknown_member)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn userset_ref_to_permission_emits_error() {
            let source = indoc! {r#"
                type group {
                  let owner = group;
                  def read = .owner;
                }

                type user {
                  let owner = group::read;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(userset_ref_to_permission)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn userset_ref_in_permission_emits_error() {
            let source = indoc! {r#"
                type group {
                  let member = user;
                }

                type user {
                  def read = group::member;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(type_ref_in_permission)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }
    }

    mod self_ref {
        use super::*;

        #[test]
        fn valid_self_ref_in_permission_no_errors() {
            let source = indoc! {r#"
                type group {}

                type user {
                  let owner = group;
                  def read = .owner;
                }
            "#};
            let reporter = run(source);
            assert!(reporter.is_clean());
        }

        #[test]
        fn self_ref_in_relation_emits_error() {
            let source = indoc! {r#"
                type user {
                  let owner = .owner;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(self_ref_in_relation)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn self_ref_unknown_relation_emits_error() {
            let source = indoc! {r#"
                type user {
                  def read = .ghost;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(unknown_relation)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }
    }

    mod traversal {
        use super::*;

        #[test]
        fn valid_traversal_no_errors() {
            let source = indoc! {r#"
                type group {
                  let member = user;
                  def read = .member;
                }

                type user {
                  let owner = group;
                  def read = .owner.read;
                }
            "#};
            let reporter = run(source);

            assert!(reporter.is_clean());
        }

        #[test]
        fn traversal_in_relation_emits_error() {
            let source = indoc! {r#"
                type user {
                  let owner = .owner.read;
                }
            "#};
            let reporter = run(source);

            assert_code(&reporter, "aes::semantic(self_ref_in_relation)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn traversal_unknown_relation_emits_error() {
            let source = indoc! {r#"
                type user {
                  def read = .ghost.read;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(unknown_relation)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn traversal_on_permission_emits_error() {
            let source = indoc! {r#"
                type user {
                  let owner = user;
                  def read = .owner;
                  def view = .read.view;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(traversal_on_permission)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn traversal_unknown_permission_on_target_type_emits_error() {
            let source = indoc! {r#"
                type group {}

                type user {
                  let owner = group;
                  def read = .owner.ghost;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(unknown_permission_on_type)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn traversal_checks_all_union_targets() {
            let source = indoc! {r#"
                type group {
                  let owner = group;
                  def read = .owner;
                }

                type org {}

                type user {
                  let owner = group | org;
                  def read = .owner.read;
                }
            "#};
            let reporter = run(source);
            assert_code(&reporter, "aes::semantic(unknown_permission_on_type)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }
    }
}
