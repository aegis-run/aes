use aes_foundation::Reporter;

use crate::{Parser, errors, token::TokenKind};

impl<'src, R: Reporter> Parser<'src, R> {
    pub(crate) fn type_def(&mut self) {
        let start = self.start_span();

        self.expect(TokenKind::KwType);
        let name = self.ident();

        let (lets, defs) = self.braced(|p| {
            let let_cp = p.ast.lets.checkpoint();
            let def_cp = p.ast.defs.checkpoint();

            loop {
                match p.token.kind() {
                    TokenKind::KwLet => p.let_member(),
                    TokenKind::KwDef => p.def_member(),
                    TokenKind::RBrace | TokenKind::Eof => break,
                    _ => {
                        p.report(errors::unexpected_token(p.token));
                        p.skip_while(|k| {
                            !matches!(k, TokenKind::KwLet | TokenKind::KwDef | TokenKind::RBrace)
                        });
                    }
                }
            }

            (p.ast.lets.since(let_cp), p.ast.defs.since(def_cp))
        });

        self.ast.type_def(self.end_span(start), name, lets, defs);
    }

    fn let_member(&mut self) {
        let start = self.start_span();

        self.expect(TokenKind::KwLet);
        let name = self.ident();

        self.expect(TokenKind::Eq);
        let expr = self.expr();
        self.semicolon();

        self.ast.let_member(self.end_span(start), name, expr);
    }

    fn def_member(&mut self) {
        let start = self.start_span();

        self.expect(TokenKind::KwDef);
        let name = self.ident();

        self.expect(TokenKind::Eq);
        let expr = self.expr();
        self.semicolon();

        self.ast.def_member(self.end_span(start), name, expr);
    }
}

#[cfg(test)]
mod tests {
    use aes_allocator::Allocator;
    use aes_ast::*;
    use indoc::indoc;

    use crate::parser::tests::parse;

    mod type_def {
        use super::*;

        #[test]
        fn empty() {
            let source = indoc! {r#"
                type user {}
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            assert_eq!(ast.types().len(), 1);
            let ty = ast.types().at(TypeDefId::new(0));

            assert_eq!(ty.name().text(source), "user");
            assert!(ty.lets().is_empty());
            assert!(ty.defs().is_empty());
        }

        #[test]
        fn multiple_empty() {
            let source = indoc! {r#"
                type user {}
                type team {}
                type org {}
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            assert_eq!(ast.types().len(), 3);
            assert_eq!(
                ast.iter_types()
                    .map(|t| t.name().text(source))
                    .collect::<Vec<_>>(),
                vec!["user", "team", "org"]
            );
        }

        #[test]
        fn with_let_members() {
            let source = indoc! {r#"
                type team {
                  let parent = organization;
                  let member = user;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            assert_eq!(ast.types().len(), 1);
            let ty = ast.types().at(TypeDefId::new(0));
            assert_eq!(ty.lets().len(), 2);
            assert!(ty.defs().is_empty());

            let lets = ast.lets().range(ty.lets());
            assert_eq!(
                lets.map(|it| it.name().text(source)).collect::<Vec<_>>(),
                vec!["parent", "member"]
            );
        }

        #[test]
        fn with_def_members() {
            let source = indoc! {r#"
                type team {
                  def member = .maintainer | .direct_member;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let ty = ast.types().at(TypeDefId::new(0));
            assert!(ty.lets().is_empty());

            assert_eq!(ty.defs().len(), 1);
            let def = ast.defs().range(ty.defs()).next().unwrap();
            assert_eq!(def.name().text(source), "member");
        }

        #[test]
        fn with_mixed_members() {
            let source = indoc! {r#"
                type repository {
                  let organization = organization;
                  let reader = user | team;

                  def push = .writer | .organization.owner;
                  def read = .clone;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let ty = ast.types().at(TypeDefId::new(0));

            assert_eq!(ty.lets().len(), 2);
            assert_eq!(
                ast.lets()
                    .range(ty.lets())
                    .map(|it| it.name().text(source))
                    .collect::<Vec<_>>(),
                vec!["organization", "reader"],
            );

            assert_eq!(ty.defs().len(), 2);
            assert_eq!(
                ast.defs()
                    .range(ty.defs())
                    .map(|it| it.name().text(source))
                    .collect::<Vec<_>>(),
                vec!["push", "read"],
            );
        }
    }

    mod let_member {
        use super::*;

        #[test]
        fn simple_type_ref() {
            let source = indoc! {r#"
                type t {
                  let x = user;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let m = ast.lets().at(LetMemberId::new(0));
            assert_eq!(m.name().text(source), "x");

            let expr = ast.exprs().at(m.expr());
            assert!(matches!(expr.term(), ExprTerm::TypeRef(s) if s.span.text(source) == "user"));
        }

        #[test]
        fn union_expr() {
            let source = indoc! {r#"
                type t {
                  let x = user | team;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let let_def = ast.lets().at(LetMemberId::new(0));
            let expr = ast.exprs().at(let_def.expr());

            let ExprTerm::Binary(expr) = expr.term() else {
                panic!("expected Binary, got {:?}", expr.term());
            };

            assert_eq!(expr.op, BinaryOp::Union);
            assert!(matches!(
                ast.exprs().at(expr.lhs).term(),
                ExprTerm::TypeRef(_)
            ));
            assert!(matches!(
                ast.exprs().at(expr.rhs).term(),
                ExprTerm::TypeRef(_)
            ));
        }

        #[test]
        fn userset_type_ref() {
            let source = indoc! {r#"
                type t {
                  let x = team::member;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let let_def = ast.lets().at(LetMemberId::new(0));
            let expr = ast.exprs().at(let_def.expr());
            let ExprTerm::UsersetTypeRef(expr) = expr.term() else {
                panic!("expected UsersetTypeRef, got {:?}", expr.term());
            };

            assert_eq!(expr.ty.text(source), "team");
            assert_eq!(expr.member.text(source), "member");
        }
    }

    mod def_member {
        use super::*;

        #[test]
        fn self_ref() {
            let source = indoc! {r#"
                type t {
                  def x = .reader;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let def = ast.defs().at(DefMemberId::new(0));
            assert_eq!(def.name().text(source), "x");

            let expr = ast.exprs().at(def.expr());
            assert!(matches!(expr.term(), ExprTerm::SelfRef(s) if s.span.text(source) == "reader"));
        }

        #[test]
        fn traversal() {
            let source = indoc! {r#"
                type t {
                  def x = .organization.owner;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            let def = ast.defs().at(DefMemberId::new(0));
            let expr = ast.exprs().at(def.expr());
            let ExprTerm::Traversal(expr) = expr.term() else {
                panic!("expected Traversal, got {:?}", expr.term());
            };

            assert_eq!(expr.relation.text(source), "organization");
            assert_eq!(expr.permission.text(source), "owner");
        }

        #[test]
        fn chained_union() {
            let source = indoc! {r#"
                type t {
                  def x = .writer | .maintainer | .admin;
                }
            "#};

            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, source);
            assert!(reporter.is_clean());

            // Union is left-associative, so: ((.writer | .maintainer) | .admin)
            let def = ast.defs().at(DefMemberId::new(0));
            let expr = ast.exprs().at(def.expr());

            let ExprTerm::Binary(expr) = expr.term() else {
                panic!("expected Binary, got {:?}", expr.term());
            };

            assert_eq!(expr.op, BinaryOp::Union);
            // rhs is .admin
            assert!(matches!(
                ast.exprs().at(expr.rhs).term(),
                ExprTerm::SelfRef(_)
            ));
            // lhs is (.writer | .maintainer)
            assert!(matches!(
                ast.exprs().at(expr.lhs).term(),
                ExprTerm::Binary(expr) if expr.op == BinaryOp::Union,
            ));
        }
    }
}
