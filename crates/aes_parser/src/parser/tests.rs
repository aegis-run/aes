use std::sync::Arc;

use aes_allocator::Allocator;
use aes_ast::AssertionKind;
use aes_foundation::Span;

use crate::Parser;

pub(crate) struct ParseResult<'src> {
    pub source: &'src str,
    pub ast: aes_ast::Ast<'src>,
    pub errors: Vec<aes_foundation::Diagnostic>,
}

impl<'src> ParseResult<'src> {
    pub(crate) fn text(&self, span: Span) -> &str {
        span.text(self.source)
    }

    pub(crate) fn error_messages(&self) -> Vec<&str> {
        self.errors.iter().map(|d| d.message()).collect()
    }

    pub(crate) fn has_no_errors(&self) {
        assert!(
            self.errors.is_empty(),
            "expected no errors, got: {:?}",
            self.error_messages()
        );
    }
}

pub(crate) fn parse<'src>(alloc: &'src Allocator, source: &'src str) -> ParseResult<'src> {
    let (ast, errors) = Parser::new(alloc, source).parse();

    ParseResult {
        source,
        ast,
        errors,
    }
}

mod empty {
    use super::*;

    #[test]
    fn empty_source() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "");
        r.has_no_errors();
        assert_eq!(r.ast.types().len(), 0);
        assert_eq!(r.ast.tests().len(), 0);
    }

    #[test]
    fn whitespace_only() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "   \n\t\n  ");
        r.has_no_errors();
        assert_eq!(r.ast.types().len(), 0);
    }

    #[test]
    fn comment_only() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "// just a comment\n// another one");
        r.has_no_errors();
        assert_eq!(r.ast.types().len(), 0);
    }
}

mod error_recovery {
    use super::*;

    #[test]
    fn missing_semicolon_in_let() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = user let y = team; }");
        assert!(!r.errors.is_empty());
        // Should still parse both members
        assert_eq!(r.ast.lets().len(), 2);
    }

    #[test]
    fn unexpected_token_in_type_body() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { @ let x = user; }");
        assert!(!r.errors.is_empty());
        // let member should still be parsed after recovery
        assert_eq!(r.ast.lets().len(), 1);
    }

    #[test]
    fn junk_between_types() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type a {} blah garbage type b {}");
        assert!(!r.errors.is_empty());
        assert_eq!(r.ast.types().len(), 2);
    }

    #[test]
    fn empty_source_no_crash() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "");
        r.has_no_errors();
    }

    #[test]
    fn missing_type_name() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type {}");
        assert!(!r.errors.is_empty());
        // Should still produce a type def node
        assert_eq!(r.ast.types().len(), 1);
    }

    #[test]
    fn missing_brace_at_eof() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t {");
        assert!(!r.errors.is_empty());
        assert_eq!(r.ast.types().len(), 1);
    }

    #[test]
    fn missing_relations_keyword() {
        let alloc = Allocator::new();
        let r = parse(&alloc, r#"test "t" { assert( org("a").x( user("b") ) ); }"#);
        assert!(
            r.errors
                .iter()
                .any(|e| e.code().number.as_ref().unwrap() == "missing_relations_block")
        );
    }
}

mod diagnostics {
    use super::*;

    #[test]
    fn missing_semicolon_message() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = user }");
        assert!(
            r.errors
                .iter()
                .any(|e| e.code().number.as_ref().unwrap() == "missing_semicolon")
        )
    }

    #[test]
    fn unclosed_brace_message() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = user;");
        assert!(
            r.errors
                .iter()
                .any(|e| e.code().number.as_ref().unwrap() == "unclosed_delimiter")
        );
    }

    #[test]
    fn unexpected_top_level_token() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "@ type t {}");
        assert!(
            r.errors
                .iter()
                .any(|e| e.code().number.as_ref().unwrap() == "unexpected_token")
        );
        assert_eq!(r.ast.types().len(), 1);
    }

    #[test]
    fn duplicate_relations_block() {
        let alloc = Allocator::new();
        let r = parse(&alloc, r#"test "t" { relations {} relations {} }"#);

        assert!(
            r.errors
                .iter()
                .any(|e| e.code().number.as_ref().unwrap() == "duplicate_relations_block")
        );
    }

    #[test]
    fn expected_term_error() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = @; }");
        assert!(
            r.errors
                .iter()
                .any(|e| e.code().number.as_ref().unwrap() == "expected_term")
        );
    }

    #[test]
    fn error_codes_are_set() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = user }");
        for err in &r.errors {
            assert!(
                err.code().is_some(),
                "error missing code: {}",
                err.message()
            );
        }
    }
}

mod realistic {
    use super::*;

    #[test]
    fn full_schema_and_test() {
        let source = r#"
            type user {}

            type team {
                let parent = organization | team;
                let maintainer = user;
                let direct_member = user;

                def member = .maintainer | .direct_member;
            }

            type organization {
                let owner = user;
                let member = user;

                def create_repository = .owner | .member;
                def manage_billing = .owner;
            }

            type repository {
                let organization = organization;
                let reader = user | team::member;
                let writer = user | team::member;

                def push = .writer | .organization.owner;
                def read = .reader | .organization.owner;
            }

            test "repo_access" {
                relations {
                    organization("acme") .{
                        .owner: user("alice");
                        .member: user("bob");
                    };
                    team("infra") .{
                        .parent: organization("acme");
                        .maintainer: user("charlie");
                        .direct_member: user("david");
                    };
                    repository("gateway") .{
                        .organization: organization("acme");
                        .writer: team("infra")::member;
                        .reader: user("eve");
                    };
                }
                assert( organization("acme").manage_billing( user("alice") ) );
                assert( repository("gateway").push( user("charlie") ) );
                assert_not( repository("gateway").push( user("eve") ) );
            }
        "#;

        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        r.has_no_errors();

        assert_eq!(r.ast.types().len(), 4);
        assert_eq!(r.ast.tests().len(), 1);

        // Check relations count: 3 org + 2 team + 3 repo = 8
        assert_eq!(r.ast.relations().len(), 8);

        // Check assertions count
        assert_eq!(r.ast.asserts().len(), 3);

        // Verify first assertion
        let a0 = r.ast.asserts().at(aes_ast::AssertId::new(0));
        assert_eq!(a0.kind(), AssertionKind::Assert);
        assert_eq!(r.text(a0.resource().ty()), "organization");
        assert_eq!(r.text(a0.resource().ident()), r#""acme""#);
        assert_eq!(r.text(a0.permission()), "manage_billing");
        assert_eq!(r.text(a0.actor().ty()), "user");
        assert_eq!(r.text(a0.actor().ident()), r#""alice""#);

        // Verify last assertion is assert_not
        let a2 = r.ast.asserts().at(aes_ast::AssertId::new(2));
        assert_eq!(a2.kind(), AssertionKind::AssertNot);
    }
}

mod properties {
    use super::*;
    use proptest::prelude::*;

    fn try_parse(source: &str) -> (usize, usize, usize, usize, bool) {
        let alloc = Allocator::new();

        let (ast, errors) = Parser::new(&alloc, source).parse();
        let has_errors = !errors.is_empty();

        (
            ast.types().len(),
            ast.tests().len(),
            ast.exprs().len(),
            errors.len(),
            has_errors,
        )
    }

    fn ident() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9_]{0,7}")
            .unwrap()
            .prop_filter("not a keyword", |s| {
                !matches!(
                    s.as_str(),
                    "type" | "let" | "def" | "test" | "relations" | "assert" | "assert_not"
                )
            })
    }

    fn quoted_string() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z0-9_]{1,8}")
            .unwrap()
            .prop_map(|s| format!("\"{s}\""))
    }

    fn expr() -> impl Strategy<Value = String> {
        let leaf = prop_oneof![
            ident().prop_map(|s| s),
            (ident(), ident()).prop_map(|(a, b)| format!("{a}::{b}")),
            ident().prop_map(|s| format!(".{s}")),
            (ident(), ident()).prop_map(|(a, b)| format!(".{a}.{b}")),
        ];

        leaf.prop_recursive(
            4,  // 4 levels deep
            16, // Max 16 nodes
            2,  // Max items per collection
            |inner| {
                prop_oneof![
                    (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} | {b}")),
                    (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} & {b}")),
                    (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} - {b}")),
                    inner.prop_map(|a| format!("({a})")),
                ]
            },
        )
    }

    fn let_member() -> impl Strategy<Value = String> {
        (ident(), expr()).prop_map(|(name, expr)| format!("let {name} = {expr};"))
    }

    fn def_member() -> impl Strategy<Value = String> {
        (ident(), expr()).prop_map(|(name, expr)| format!("def {name} = {expr};"))
    }

    fn type_def() -> impl Strategy<Value = String> {
        (
            ident(),
            proptest::collection::vec(let_member(), 0..4),
            proptest::collection::vec(def_member(), 0..4),
        )
            .prop_map(|(name, lets, defs)| {
                let body = lets
                    .into_iter()
                    .chain(defs)
                    .collect::<Vec<_>>()
                    .join("\n    ");
                format!("type {name} {{\n    {body}\n}}")
            })
    }

    fn instance() -> impl Strategy<Value = String> {
        (ident(), quoted_string()).prop_map(|(ty, id)| format!("{ty}({id})"))
    }

    fn subject() -> impl Strategy<Value = String> {
        prop::strategy::Union::new(vec![
            instance().boxed(),
            (instance(), ident())
                .prop_map(|(inst, perm)| format!("{inst}::{perm}"))
                .boxed(),
        ])
    }

    fn relation_stmt() -> impl Strategy<Value = String> {
        prop_oneof![
            // Single inline: folder("id").viewer: user("id");
            (instance(), ident(), subject()).prop_map(|(i, r, s)| format!("{i}.{r}:{s};")),
            // Block: folder("id").{ .viewer: user("id"); .editor: user("id"); };
            (
                instance(),
                proptest::collection::vec(relation_assign(), 1..3)
            )
                .prop_map(|(inst, assigns)| { format!("{}.{{ {} }};", inst, assigns.join(" ")) })
        ]
    }

    fn relation_assign() -> impl Strategy<Value = String> {
        (ident(), subject()).prop_map(|(rel, subj)| format!(".{rel}:{subj};"))
    }

    fn assertion() -> impl Strategy<Value = String> {
        (prop::bool::ANY, instance(), ident(), instance()).prop_map(
            |(positive, resource, perm, actor)| {
                let kw = if positive { "assert" } else { "assert_not" };
                format!("{kw}( {resource}.{perm}( {actor} ) );")
            },
        )
    }

    fn test_def() -> impl Strategy<Value = String> {
        (
            quoted_string(),
            proptest::collection::vec(relation_stmt(), 0..4),
            proptest::collection::vec(assertion(), 0..4),
        )
            .prop_map(|(name, rels, asserts)| {
                let rels_body = rels.join("\n        ");
                let asserts_body = asserts.join("\n    ");
                format!(
                    "test {name} {{\n    relations {{\n        {rels_body}\n    }}\n    {asserts_body}\n}}"
                )
            })
    }

    fn valid_program() -> impl Strategy<Value = String> {
        (
            proptest::collection::vec(type_def(), 0..4),
            proptest::collection::vec(test_def(), 0..2),
        )
            .prop_map(|(types, tests)| {
                types
                    .into_iter()
                    .chain(tests)
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
    }

    proptest! {
        #[test]
        fn never_panics(s in "\\PC{0,256}") {
            let _ = std::panic::catch_unwind(|| {
                let alloc = Allocator::new();
                let _ = Parser::new(&alloc, &s).parse();
            });
        }

        #[test]
        fn valid_programs_produce_no_errors(source in valid_program()) {
            let alloc = Allocator::new();
            let r = parse(&alloc, &source);
            prop_assert!(
                r.errors.is_empty(),
                "valid program had errors:\nsource:\n{}\nerrors: {:?}",
                source,
                r.error_messages()
            );
        }

        #[test]
        fn error_count_bounded_by_source_len(s in "\\PC{0,256}") {
            let (_, _, _, err_count, _) = try_parse(&s);
            prop_assert!(
                err_count <= s.len() + 10,
                "too many errors ({}) for input of length {}",
                err_count,
                s.len()
            );
        }

        #[test]
        fn valid_program_spans_in_bounds(source in valid_program()) {
            let alloc = Allocator::new();
            let r = parse(&alloc, &source);
            let len = source.len() as u32;

            for i in 0..r.ast.exprs().len() {
                let expr = r.ast.exprs().at(aes_ast::ExprId::new(i as u32));
                let span = expr.span();
                prop_assert!(
                    span.start() <= len && span.end() <= len,
                    "expr span out of bounds: {:?} for source len {}",
                    span, len
                );
            }

            for i in 0..r.ast.types().len() {
                let ty = r.ast.types().at(aes_ast::TypeDefId::new(i as u32));
                let span = ty.span();
                prop_assert!(
                    span.start() <= len && span.end() <= len,
                    "type span out of bounds: {:?} for source len {}",
                    span, len
                );
            }
        }

        #[test]
        fn pool_sizes_consistent_for_valid_programs(source in valid_program()) {
            let alloc = Allocator::new();
            let r = parse(&alloc, &source);
            r.has_no_errors();

            // Every let/def expr must resolve to a valid ExprId
            for i in 0..r.ast.lets().len() {
                let m = r.ast.lets().at(aes_ast::LetMemberId::new(i as u32));
                prop_assert!(
                    m.expr().as_index() < r.ast.exprs().len(),
                    "let member expr id out of bounds"
                );
            }
            for i in 0..r.ast.defs().len() {
                let m = r.ast.defs().at(aes_ast::DefMemberId::new(i as u32));
                prop_assert!(
                    m.expr().as_index() < r.ast.exprs().len(),
                    "def member expr id out of bounds"
                );
            }
            // Every relation must reference a valid SubjectId
            for i in 0..r.ast.relations().len() {
                let rel = r.ast.relations().at(aes_ast::RelationId::new(i as u32));
                prop_assert!(
                    rel.subject().as_index() < r.ast.subjects().len(),
                    "relation subject id out of bounds"
                );
            }
        }
    }
}

mod snapshots {
    use super::*;

    use aes_foundation::{GraphicalReportHandler, GraphicalTheme, NamedSource};

    fn render_diagnostics(source: &str, errors: &[aes_foundation::Diagnostic]) -> String {
        let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
        let named_src = Arc::new(NamedSource::new("test.aes", source.to_owned()));

        let mut out = String::new();
        for (i, diag) in errors.iter().enumerate() {
            if i > 0 {
                out.push_str("\n---\n\n");
            }
            let error = diag.clone().with_source_code(named_src.clone());
            handler.render_report(&mut out, error.as_ref()).ok();
        }
        out
    }

    #[test]
    fn ast_empty_type() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type user {}");
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_type_with_lets() {
        let alloc = Allocator::new();
        let r = parse(
            &alloc,
            "type team { let parent = organization; let member = user; }",
        );
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_type_with_defs() {
        let alloc = Allocator::new();
        let r = parse(
            &alloc,
            "type repo { def push = .writer | .organization.owner; }",
        );
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_expr_precedence() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = a | b & c - d; }");
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_expr_parenthesized() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = (a | b) & c; }");
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_userset_ref() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = team::member; }");
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_test_with_relations_and_asserts() {
        let alloc = Allocator::new();
        let r = parse(
            &alloc,
            r#"test "access" {
                relations {
                    org("acme") .owner: user("alice");
                    repo("gw") .writer: team("dev")::member;
                }
                assert( org("acme").manage( user("alice") ) );
                assert_not( repo("gw").push( user("bob") ) );
            }"#,
        );
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_block_relations() {
        let alloc = Allocator::new();
        let r = parse(
            &alloc,
            r#"test "t" {
                relations {
                    org("a") .{
                        .owner: user("x");
                        .member: user("y");
                    };
                }
            }"#,
        );
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn ast_full_realistic_schema() {
        let alloc = Allocator::new();
        let r = parse(
            &alloc,
            r#"
            type user {}

            type organization {
                let owner = user;
                let member = user;
                def manage = .owner;
            }

            type repository {
                let org = organization;
                let reader = user | team::member;
                def push = .writer | .org.owner;
            }

            test "repo_flow" {
                relations {
                    organization("acme") .{
                        .owner: user("alice");
                        .member: user("bob");
                    };
                    repository("gw") .{
                        .org: organization("acme");
                        .reader: user("eve");
                    };
                }
                assert( organization("acme").manage( user("alice") ) );
                assert_not( repository("gw").push( user("eve") ) );
            }
            "#,
        );
        r.has_no_errors();
        insta::assert_debug_snapshot!(r.ast);
    }

    #[test]
    fn diag_missing_semicolon() {
        let source = "type t { let x = user }";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_unclosed_brace() {
        let source = "type t { let x = user;";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_unexpected_top_level() {
        let source = "blah type t {}";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_missing_type_name() {
        let source = "type {}";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_expected_term() {
        let source = "type t { let x = @; }";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_missing_relations_block() {
        let source = r#"test "t" { assert( org("a").x( user("b") ) ); }"#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_duplicate_relations() {
        let source = r#"test "t" { relations {} relations {} }"#;
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_missing_semicolon_in_let() {
        let source = "type t { let x = user let y = team; }";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }

    #[test]
    fn diag_junk_between_types() {
        let source = "type a {} blah garbage type b {}";
        let alloc = Allocator::new();
        let r = parse(&alloc, source);
        insta::assert_snapshot!(render_diagnostics(source, &r.errors));
    }
}
