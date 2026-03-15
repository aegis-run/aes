use crate::{Parser, errors, token::TokenKind};

impl<'src> Parser<'src> {
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
                        p.errors.push(errors::unexpected_token(p.token));
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

    use crate::parser::tests::parse;

    mod type_def {
        use super::*;

        #[test]
        fn empty() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type user {}");
            r.has_no_errors();
            assert_eq!(r.ast.types().len(), 1);

            let ty = r.ast.types().at(TypeDefId::new(0));
            assert_eq!(r.text(ty.name()), "user");
            assert!(ty.lets().is_empty());
            assert!(ty.defs().is_empty());
        }

        #[test]
        fn multiple_empty() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type user {} type team {} type org {}");
            r.has_no_errors();
            assert_eq!(r.ast.types().len(), 3);

            assert_eq!(r.text(r.ast.types().at(TypeDefId::new(0)).name()), "user");
            assert_eq!(r.text(r.ast.types().at(TypeDefId::new(1)).name()), "team");
            assert_eq!(r.text(r.ast.types().at(TypeDefId::new(2)).name()), "org");
        }

        #[test]
        fn with_let_members() {
            let alloc = Allocator::new();
            let r = parse(
                &alloc,
                "type team { let parent = organization; let member = user; }",
            );
            r.has_no_errors();
            assert_eq!(r.ast.types().len(), 1);

            let ty = r.ast.types().at(TypeDefId::new(0));
            assert_eq!(ty.lets().len(), 2);
            assert!(ty.defs().is_empty());

            let lets: Vec<_> = r.ast.lets().range(ty.lets()).collect();
            assert_eq!(r.text(lets[0].name()), "parent");
            assert_eq!(r.text(lets[1].name()), "member");
        }

        #[test]
        fn with_def_members() {
            let alloc = Allocator::new();
            let r = parse(
                &alloc,
                "type team { def member = .maintainer | .direct_member; }",
            );
            r.has_no_errors();

            let ty = r.ast.types().at(TypeDefId::new(0));
            assert!(ty.lets().is_empty());
            assert_eq!(ty.defs().len(), 1);

            let def = r.ast.defs().range(ty.defs()).next().unwrap();
            assert_eq!(r.text(def.name()), "member");
        }

        #[test]
        fn with_mixed_members() {
            let alloc = Allocator::new();
            let r = parse(
                &alloc,
                "type repository {
                let organization = organization;
                let reader = user | team;
                def push = .writer | .organization.owner;
                def read = .clone;
            }",
            );
            r.has_no_errors();

            let ty = r.ast.types().at(TypeDefId::new(0));
            assert_eq!(ty.lets().len(), 2);
            assert_eq!(ty.defs().len(), 2);
        }
    }

    mod let_member {
        use super::*;

        #[test]
        fn simple_type_ref() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type t { let x = user; }");
            r.has_no_errors();

            let m = r.ast.lets().at(LetMemberId::new(0));
            assert_eq!(r.text(m.name()), "x");

            let expr = r.ast.exprs().at(m.expr());
            assert!(matches!(expr.term(), ExprTerm::TypeRef(s) if r.text(s.span) == "user"));
        }

        #[test]
        fn union_expr() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type t { let x = user | team; }");
            r.has_no_errors();

            let let_def = r.ast.lets().at(LetMemberId::new(0));
            let expr = r.ast.exprs().at(let_def.expr());

            let ExprTerm::Binary(expr) = expr.term() else {
                panic!("expected Binary, got {:?}", expr.term());
            };

            assert_eq!(expr.op, BinaryOp::Union);
            assert!(matches!(
                r.ast.exprs().at(expr.lhs).term(),
                ExprTerm::TypeRef(_)
            ));
            assert!(matches!(
                r.ast.exprs().at(expr.rhs).term(),
                ExprTerm::TypeRef(_)
            ));
        }

        #[test]
        fn userset_type_ref() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type t { let x = team::member; }");
            r.has_no_errors();

            let let_def = r.ast.lets().at(LetMemberId::new(0));
            let expr = r.ast.exprs().at(let_def.expr());
            let ExprTerm::UsersetTypeRef(expr) = expr.term() else {
                panic!("expected UsersetTypeRef, got {:?}", expr.term());
            };

            assert_eq!(r.text(expr.ty), "team");
            assert_eq!(r.text(expr.member), "member");
        }
    }

    mod def_member {
        use super::*;

        #[test]
        fn self_ref() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type t { def x = .reader; }");
            r.has_no_errors();

            let def = r.ast.defs().at(DefMemberId::new(0));
            assert_eq!(r.text(def.name()), "x");

            let expr = r.ast.exprs().at(def.expr());
            assert!(matches!(expr.term(), ExprTerm::SelfRef(s) if r.text(s.span) == "reader"));
        }

        #[test]
        fn traversal() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type t { def x = .organization.owner; }");
            r.has_no_errors();

            let def = r.ast.defs().at(DefMemberId::new(0));
            let expr = r.ast.exprs().at(def.expr());
            let ExprTerm::Traversal(expr) = expr.term() else {
                panic!("expected Traversal, got {:?}", expr.term());
            };

            assert_eq!(r.text(expr.relation), "organization");
            assert_eq!(r.text(expr.permission), "owner");
        }

        #[test]
        fn chained_union() {
            let alloc = Allocator::new();
            let r = parse(&alloc, "type t { def x = .writer | .maintainer | .admin; }");
            r.has_no_errors();

            // Union is left-associative, so: ((.writer | .maintainer) | .admin)
            let def = r.ast.defs().at(DefMemberId::new(0));
            let expr = r.ast.exprs().at(def.expr());

            let ExprTerm::Binary(expr) = expr.term() else {
                panic!("expected Binary, got {:?}", expr.term());
            };

            assert_eq!(expr.op, BinaryOp::Union);
            // rhs is .admin
            assert!(matches!(
                r.ast.exprs().at(expr.rhs).term(),
                ExprTerm::SelfRef(_)
            ));
            // lhs is (.writer | .maintainer)
            assert!(matches!(
                r.ast.exprs().at(expr.lhs).term(),
                ExprTerm::Binary(expr) if expr.op == BinaryOp::Union,
            ));
        }
    }
}
