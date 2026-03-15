use aes_ast::{AssertRange, Instance, RelationRange, SubjectId};
use aes_foundation::Span;

use crate::{Parser, errors, token::TokenKind};

impl<'src> Parser<'src> {
    pub(crate) fn test_def(&mut self) {
        let start = self.start_span();

        self.expect(TokenKind::KwTest);
        let name = self.string();

        self.braced(|p| {
            let Some(relations) = p.relations(start, name) else {
                return;
            };

            let asserts = p.asserts();

            p.ast.test_def(p.end_span(start), name, relations, asserts);
        });
    }

    fn relations(&mut self, start: u32, name: Span) -> Option<RelationRange> {
        if !self.eat(TokenKind::KwRelations) {
            self.errors
                .push(errors::missing_relations_block(self.token));

            self.ast.test_def(
                self.end_span(start),
                name,
                self.ast.relations.empty_range(),
                self.ast.asserts.empty_range(),
            );
            return None;
        }

        self.braced(|p| {
            let cp = p.ast.relations.checkpoint();

            loop {
                match p.token.kind() {
                    TokenKind::RBrace | TokenKind::Eof => break,
                    TokenKind::Ident => p.relation_stmt(),
                    TokenKind::KwAssert | TokenKind::KwAssertNot => {
                        p.errors.push(errors::assert_before_relations(p.token));
                        break;
                    }
                    _ => {
                        p.errors.push(errors::unexpected_token(p.token));
                        p.skip_while(|k| !matches!(k, TokenKind::Ident | TokenKind::RBrace));
                    }
                }
            }

            Some(p.ast.relations.since(cp))
        })
    }

    fn relation_stmt(&mut self) {
        let resource = self.instance();
        self.expect(TokenKind::Dot);

        match self.token.kind() {
            TokenKind::LBrace => {
                self.braced(|p| {
                    loop {
                        match p.token.kind() {
                            TokenKind::RBrace | TokenKind::Eof => break,
                            TokenKind::Dot => {
                                p.expect(TokenKind::Dot);
                                p.relation_assign(resource);
                            }

                            _ => {
                                p.errors
                                    .push(errors::expected_relation_name_or_block(p.token));
                                p.skip_while(|k| !matches!(k, TokenKind::Dot | TokenKind::RBrace));
                            }
                        }
                    }
                });
                self.semicolon();
            }

            TokenKind::Ident => self.relation_assign(resource),

            _ => {
                self.errors.push(errors::unexpected_token(self.token));
                self.skip_while(|k| !matches!(k, TokenKind::Ident | TokenKind::RBrace));
            }
        }
    }

    fn relation_assign(&mut self, resource: Instance) {
        let start = self.start_span();

        let relation = self.ident();

        self.expect(TokenKind::Colon);
        let subject = self.subject();
        self.semicolon();

        self.ast
            .relation(self.end_span(start), resource, relation, subject);
    }

    fn asserts(&mut self) -> AssertRange {
        let cp = self.ast.asserts.checkpoint();

        loop {
            match self.token.kind() {
                TokenKind::RBrace | TokenKind::Eof => break,
                TokenKind::KwAssert | TokenKind::KwAssertNot => self.assert_stmt(),
                TokenKind::KwRelations => {
                    self.errors
                        .push(errors::duplicate_relations_block(self.token));

                    self.skip();
                    self.braced(|p| {
                        while !p.at(TokenKind::RBrace) && !p.at(TokenKind::Eof) {
                            p.skip();
                        }
                    });
                }
                _ => {
                    self.errors.push(errors::unexpected_token(self.token));
                    self.skip_while(|k| {
                        !matches!(
                            k,
                            TokenKind::KwAssert
                                | TokenKind::KwAssertNot
                                | TokenKind::KwRelations
                                | TokenKind::RBrace
                        )
                    });
                }
            }
        }

        self.ast.asserts.since(cp)
    }

    fn assert_stmt(&mut self) {
        let start = self.start_span();

        let kind = match self.token.kind() {
            TokenKind::KwAssert => {
                self.skip();
                aes_ast::AssertionKind::Assert
            }
            TokenKind::KwAssertNot => {
                self.skip();
                aes_ast::AssertionKind::AssertNot
            }
            _ => {
                self.errors.push(errors::unexpected_token(self.token));
                return;
            }
        };

        let (resource, permission, actor) = self.parenthesized(|p| {
            let resource = p.instance();

            p.expect(TokenKind::Dot);
            let permission = p.ident();

            let actor = p.parenthesized(|p| p.instance());

            (resource, permission, actor)
        });
        self.semicolon();

        self.ast
            .assert(self.end_span(start), kind, resource, permission, actor);
    }

    #[must_use]
    fn subject(&mut self) -> SubjectId {
        let start = self.start_span();

        let instance = self.instance();
        let permission = if self.eat(TokenKind::ColonColon) {
            if !self.at(TokenKind::Ident) {
                self.errors
                    .push(errors::expected_permission_after_colons(self.token));
            }
            Some(self.ident())
        } else {
            None
        };

        self.ast.subject(self.end_span(start), instance, permission)
    }

    #[must_use]
    fn instance(&mut self) -> aes_ast::Instance {
        if !self.at(TokenKind::Ident) {
            self.errors.push(errors::expected_type_name(self.token));
        }
        let ty = self.ident();
        let ident = self.parenthesized(|p| p.string());

        aes_ast::Instance::new(ty, ident)
    }
}

#[cfg(test)]
mod tests {
    use aes_allocator::Allocator;
    use aes_ast::*;

    use crate::parser::tests::parse;

    #[test]
    fn basic() {
        let alloc = Allocator::new();
        let r = parse(&alloc, r#"test "basic" { relations {} }"#);
        r.has_no_errors();
        assert_eq!(r.ast.tests().len(), 1);

        let t = r.ast.tests().at(TestDefId::new(0));
        assert_eq!(r.text(t.name()), r#""basic""#);
        assert!(t.relations().is_empty());
        assert!(t.asserts().is_empty());
    }

    #[test]
    fn inline_relation() {
        let source = r#"test "t" { relations { org("acme") .owner: user("alice"); } }"#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        r.has_no_errors();

        assert_eq!(r.ast.relations().len(), 1);
        let rel = r.ast.relations().at(RelationId::new(0));
        assert_eq!(r.text(rel.resource().ty()), "org");
        assert_eq!(r.text(rel.resource().ident()), r#""acme""#);
        assert_eq!(r.text(rel.relation()), "owner");

        let subj = r.ast.subjects().at(rel.subject());
        assert_eq!(r.text(subj.instance().ty()), "user");
        assert_eq!(r.text(subj.instance().ident()), r#""alice""#);
        assert!(subj.permission().is_none());
    }

    #[test]
    fn block_relations() {
        let source = r#"test "t" {
            relations {
                org("acme") .{
                    .owner: user("alice");
                    .member: user("bob");
                };
            }
        }"#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        r.has_no_errors();
        assert_eq!(r.ast.relations().len(), 2);

        assert_eq!(
            r.text(r.ast.relations().at(RelationId::new(0)).relation()),
            "owner"
        );
        assert_eq!(
            r.text(r.ast.relations().at(RelationId::new(1)).relation()),
            "member"
        );
    }

    #[test]
    fn subject_with_permission() {
        let source = r#"test "t" { relations { repo("x") .writer: team("dev")::member; } }"#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        r.has_no_errors();

        let rel = r.ast.relations().at(RelationId::new(0));
        let subj = r.ast.subjects().at(rel.subject());

        assert_eq!(r.text(subj.instance().ty()), "team");
        assert!(subj.permission().is_some());
        assert_eq!(r.text(subj.permission().unwrap()), "member");
    }

    #[test]
    fn with_assertions() {
        let source = r#"test "t" {
            relations { org("a") .owner: user("alice"); }
            assert( org("a").manage( user("alice") ) );
            assert_not( org("a").delete( user("bob") ) );
        }"#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        r.has_no_errors();

        assert_eq!(r.ast.asserts().len(), 2);

        let a0 = r.ast.asserts().at(AssertId::new(0));
        assert_eq!(a0.kind(), AssertionKind::Assert);
        assert_eq!(r.text(a0.permission()), "manage");

        let a1 = r.ast.asserts().at(AssertId::new(1));
        assert_eq!(a1.kind(), AssertionKind::AssertNot);
        assert_eq!(r.text(a1.permission()), "delete");
    }

    #[test]
    fn types_and_tests_together() {
        let source = r#"
            type user {}
            type org { let owner = user; }
            test "flow" {
                relations { org("a") .owner: user("x"); }
                assert( org("a").owner( user("x") ) );
            }
        "#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        r.has_no_errors();
        assert_eq!(r.ast.types().len(), 2);
        assert_eq!(r.ast.tests().len(), 1);
    }
}
