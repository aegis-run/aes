use aes_ast::Ast;
use aes_foundation::Reporter;

use crate::{Context, errors};

pub(crate) fn declare_schema<'src, R: Reporter>(ctx: &mut Context<'src, R>, ast: &'src Ast<'src>) {
    aes_visit::schema(&mut Declarer {
        ctx,
        ast,
        scope: None,
    });
}

struct Declarer<'c, 'src, R: Reporter> {
    ast: &'src Ast<'src>,
    ctx: &'c mut Context<'src, R>,

    scope: Option<aes_ast::TypeDefId>,
}

impl<'c, 'src, R: Reporter> aes_visit::Visitor<'src> for Declarer<'c, 'src, R> {
    fn ast(&self) -> &'src Ast<'src> {
        self.ast
    }

    fn type_def(&mut self, id: aes_ast::TypeDefId) {
        let scope = self.ast().types().at(id);
        let span = scope.name();
        let name = scope.name().text(self.ctx.source);

        if let Some(prev) = self.ctx.index.declare_type(id, span, name) {
            return self.ctx.report(errors::duplicate_type(span, prev, name));
        };

        self.scope = Some(id);
        aes_visit::walk_type_def(self, id);
        self.scope = None;
    }

    fn let_member(&mut self, id: aes_ast::LetMemberId) {
        let scope = self.scope();
        let rel = self.ast().lets().at(id);

        let span = rel.name();
        let name = span.text(self.ctx.source);

        if let Some(prev) = self
            .ctx
            .index
            .declare_relation(scope, span, name, rel.expr())
        {
            self.ctx
                .report(errors::duplicate_relation(span, prev, name));
        }

        if let Some(perm_span) = self.ctx.index.permission_collision(scope, name) {
            self.ctx.report(errors::relation_permission_name_collision(
                perm_span, span, name,
            ));
        }
    }

    fn def_member(&mut self, id: aes_ast::DefMemberId) {
        let scope = self.scope();
        let perm = self.ast().defs().at(id);

        let span = perm.name();
        let name = span.text(self.ctx.source);

        if let Some(prev) = self
            .ctx
            .index
            .declare_permission(scope, span, name, perm.expr())
        {
            self.ctx
                .report(errors::duplicate_permission(span, prev, name));
        }

        if let Some(rel) = self.ctx.index.relation_collision(scope, name) {
            self.ctx
                .report(errors::relation_permission_name_collision(span, rel, name));
        }
    }
}

impl<'c, 'src, R: Reporter> Declarer<'c, 'src, R> {
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
}

#[cfg(test)]
mod tests {
    use aes_allocator::Allocator;
    use aes_testing::assert_code;
    use indoc::indoc;

    use super::*;

    fn run(source: &str) -> aes_testing::Reporter {
        let alloc = Allocator::new();
        let (ast, errs) = aes_parser::Parser::new(&alloc, source).parse();
        assert!(errs.is_empty());

        let mut ctx = Context::new(&alloc, source, 8, aes_testing::Reporter::default());
        declare_schema(&mut ctx, &ast);

        ctx.reporter
    }

    mod declare_type {
        use super::*;

        #[test]
        fn single_type_no_errors() {
            let source = indoc! {r#"
                type user {}
            "#};
            let reporter = run(source);

            assert!(reporter.is_clean());
        }

        #[test]
        fn two_distinct_types_no_errors() {
            let source = indoc! {r#"
                type user {}
                type group {}
            "#};
            let reporter = run(source);
            assert!(reporter.is_clean());
        }

        #[test]
        fn duplicate_type_emits_error() {
            let source = indoc! {r#"
              type user {}
              type user {}
            "#};
            let reporter = run(&source);

            assert_code(&reporter, "aes::semantic(duplicate_type)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ))
        }

        #[test]
        fn duplicate_type_stops_walking_members() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  let member = dummy;
                }

                type user {
                  let member = dummy;
                }
            "#};

            let reporter = run(&source);
            assert_code(&reporter, "aes::semantic(duplicate_type)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ))
        }
    }

    mod declare_relation {
        use super::*;

        #[test]
        fn single_relation_no_errors() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  let member = dummy;
                }
            "#};
            let reporter = run(source);

            println!("{:?}", reporter.diagnostics);
            assert!(reporter.is_clean());
        }

        #[test]
        fn duplicate_relation_in_same_type_emits_error() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  let member = dummy;
                  let member = dummy;
                }
            "#};
            let reporter = run(&source);

            assert_code(&reporter, "aes::semantic(duplicate_relation)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn same_relation_name_in_different_types_no_errors() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  let member = dummy;
                }

                type group {
                  let member = dummy;
                }
            "#};
            let reporter = run(source);
            assert!(reporter.is_clean());
        }
    }

    mod declare_permission {
        use super::*;

        #[test]
        fn single_permission_no_errors() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  def read = dummy;
                }
            "#};
            let reporter = run(source);

            assert!(reporter.is_clean());
        }

        #[test]
        fn duplicate_permission_in_same_type_emits_error() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  def read = dummy;
                  def read = dummy;
                }
            "#};
            let reporter = run(source);

            assert_code(&reporter, "aes::semantic(duplicate_permission)");
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn same_permission_name_in_different_types_no_errors() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  def read = dummy;
                }

                type group {
                  def read = dummy;
                }
            "#};
            let reporter = run(source);

            assert!(reporter.is_clean());
        }
    }

    mod scope_collision {
        use super::*;

        #[test]
        fn relation_then_permission_same_name_emits_error() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  let read = dummy;
                  def read = dummy;
                }
            "#};
            let reporter = run(source);

            assert_code(
                &reporter,
                "aes::semantic(relation_permission_name_collision)",
            );
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn permission_then_relation_same_name_emits_error() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  def read = dummy;
                  let read = dummy;
                }
            "#};
            let reporter = run(source);

            assert_code(
                &reporter,
                "aes::semantic(relation_permission_name_collision)",
            );
            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn collision_only_within_same_type() {
            let source = indoc! {r#"
                type dummy {}

                type user {
                  let read = dummy;
                }

                type group {
                  def read = dummy;
                }
            "#};
            let reporter = run(source);

            assert!(reporter.is_clean());
        }
    }
}
