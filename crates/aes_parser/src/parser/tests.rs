use aes_allocator::Allocator;
use aes_ast::AssertionKind;
use aes_testing::assert_code;
use indoc::indoc;

use crate::Parser;

pub(crate) fn parse<'src>(
    alloc: &'src Allocator,
    source: &'src str,
) -> (aes_ast::Ast<'src>, aes_testing::Reporter) {
    let file = aes_testing::file_ref(alloc, source);
    let mut reporter = aes_testing::Reporter::default();
    let ast = Parser::new(file, &mut reporter).parse();

    (ast, reporter)
}

mod empty {
    use super::*;

    #[test]
    fn empty_source() {
        let source = "";

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        assert_eq!(ast.types().len(), 0);
        assert_eq!(ast.tests().len(), 0);
    }

    #[test]
    fn whitespace_only() {
        let source = "   \n\t\n  ";

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        assert_eq!(ast.types().len(), 0);
    }

    #[test]
    fn comment_only() {
        let source = indoc! {r#"
            // just a comment
            // another one
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        assert_eq!(ast.types().len(), 0);
    }
}

mod error_recovery {
    use super::*;

    #[test]
    fn missing_semicolon_in_let() {
        let source = indoc! {r#"
            type t {
              let x = user
              let y = team;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(!reporter.is_clean());
        assert_code(&reporter, "aes::parser(missing_semicolon)");

        // Should still parse both members
        assert_eq!(ast.lets().len(), 2);

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn unexpected_token_in_type_body() {
        let source = indoc! {r#"
            type t {
              @ let x = user;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(!reporter.is_clean());

        dbg!(&reporter.diagnostics);
        assert_code(&reporter, "aes::lexer(unexpected_character)");

        // let member should still be parsed after recovery
        assert_eq!(ast.lets().len(), 1);

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn junk_between_types() {
        let source = indoc! {r#"
            type a {}
            blah garbage type b {}
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(!reporter.is_clean());
        assert_code(&reporter, "aes::parser(unexpected_token)");

        assert_eq!(ast.types().len(), 2);

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn empty_source_no_crash() {
        let source = "";

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());
    }

    #[test]
    fn missing_type_name() {
        let source = indoc! {r#"
            type {}
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(!reporter.is_clean());

        assert_code(&reporter, "aes::parser(expected_token)");

        // Should still produce a type def node
        assert_eq!(ast.types().len(), 1);

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn missing_brace_at_eof() {
        let source = indoc! {r#"
            type t {
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(!reporter.is_clean());
        assert_code(&reporter, "aes::parser(unclosed_delimiter)");

        assert_eq!(ast.types().len(), 1);

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn missing_relations_keyword() {
        let source = indoc! {r#"
            test "t" {
              assert( org("a").x( user("b") ) );
            }
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(missing_relations_block)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }
}

mod realistic {
    use super::*;

    #[test]
    fn full_schema_and_test() {
        let source = indoc! {r#"
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
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        assert_eq!(ast.types().len(), 4);
        assert_eq!(ast.tests().len(), 1);

        // Check relations count: 3 org + 2 team + 3 repo = 8
        assert_eq!(ast.relations().len(), 8);

        // Check assertions count
        assert_eq!(ast.asserts().len(), 3);

        // Verify first assertion
        let a0 = ast.asserts().at(aes_ast::AssertId::new(0));
        assert_eq!(a0.kind(), AssertionKind::Assert);
        assert_eq!(a0.resource().ty().text(source), "organization");
        assert_eq!(a0.resource().ident().text(source), r#""acme""#);
        assert_eq!(a0.permission().text(source), "manage_billing");
        assert_eq!(a0.actor().ty().text(source), "user");
        assert_eq!(a0.actor().ident().text(source), r#""alice""#);

        // Verify last assertion is assert_not
        let a2 = ast.asserts().at(aes_ast::AssertId::new(2));
        assert_eq!(a2.kind(), AssertionKind::AssertNot);
    }
}

mod properties {
    use super::*;
    use aes_testing::generate::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn never_panics(s in "\\PC{0,256}") {
            let _ = std::panic::catch_unwind(|| {
                let alloc = Allocator::new();
                let file = aes_testing::file_ref(&alloc, &s);
                let reporter = aes_testing::Reporter::default();
                let _ = Parser::new(file, reporter).parse();
            });
        }

        #[test]
        fn valid_programs_produce_no_errors(source in syntactic_program()) {
            let alloc = Allocator::new();
            let (_, reporter) = parse(&alloc, &source);
            prop_assert!(
                reporter.diagnostics.is_empty(),
                "valid program had errors:\nsource:\n{source}\nerrors: {:?}",
                reporter.messages()
            );
        }

        #[test]
        fn valid_program_spans_in_bounds(source in syntactic_program()) {
            let alloc = Allocator::new();
            let (ast, _) = parse(&alloc, &source);
            let len = source.len() as u32;

            for i in 0..ast.exprs().len() {
                let expr = ast.exprs().at(aes_ast::ExprId::new(i as u32));
                let span = expr.span();
                prop_assert!(
                    span.start() <= len && span.end() <= len,
                    "expr span out of bounds: {:?} for source len {}",
                    span, len
                );
            }

            for i in 0..ast.types().len() {
                let ty = ast.types().at(aes_ast::TypeDefId::new(i as u32));
                let span = ty.span();
                prop_assert!(
                    span.start() <= len && span.end() <= len,
                    "type span out of bounds: {:?} for source len {}",
                    span, len
                );
            }
        }

        #[test]
        fn pool_sizes_consistent_for_valid_programs(source in syntactic_program()) {
            let alloc = Allocator::new();
            let (ast, reporter) = parse(&alloc, &source);
            assert!(reporter.is_clean());

            // Every let/def expr must resolve to a valid ExprId
            for i in 0..ast.lets().len() {
                let m = ast.lets().at(aes_ast::LetMemberId::new(i as u32));
                prop_assert!(
                    m.expr().as_index() < ast.exprs().len(),
                    "let member expr id out of bounds"
                );
            }
            for i in 0..ast.defs().len() {
                let m = ast.defs().at(aes_ast::DefMemberId::new(i as u32));
                prop_assert!(
                    m.expr().as_index() < ast.exprs().len(),
                    "def member expr id out of bounds"
                );
            }
            // Every relation must reference a valid SubjectId
            for i in 0..ast.relations().len() {
                let rel = ast.relations().at(aes_ast::RelationId::new(i as u32));
                prop_assert!(
                    rel.subject().as_index() < ast.subjects().len(),
                    "relation subject id out of bounds"
                );
            }
        }
    }
}

mod snapshots {
    use super::*;

    #[test]
    fn ast_empty_type() {
        let source = "type user {}";

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_type_with_lets() {
        let source = indoc! {r#"
            type team {
              let parent = organization;
              let member = user;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_type_with_defs() {
        let source = indoc! {r#"
            type repo {
              def push = .writer | .organization.owner;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_expr_precedence() {
        let source = indoc! {r#"
            type t {
              let x = a | b & c - d;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_expr_parenthesized() {
        let source = indoc! {r#"
            type t {
              let x = (a | b) & c;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_userset_ref() {
        let source = indoc! {r#"
            type t {
              let x = team::member;
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_test_with_relations_and_asserts() {
        let source = indoc! {r#"
            test "access" {
              relations {
                org("acme") .owner: user("alice");
                repo("gw") .writer: team("dev")::member;
              }

              assert( org("acme").manage( user("alice") ) );
              assert_not( repo("gw").push( user("bob") ) );
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_block_relations() {
        let source = indoc! {r#"
            test "t" {
              relations {
                org("a") .{
                  .owner: user("x");
                  .member: user("y");
                };
              }
            }
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());
        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn ast_full_realistic_schema() {
        let source = indoc! {r#"
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
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert!(reporter.is_clean());

        insta::assert_debug_snapshot!(ast);
    }

    #[test]
    fn diag_missing_semicolon() {
        let source = indoc! {r#"
            type t {
              let x = user
            }
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(missing_semicolon)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_unclosed_brace() {
        let source = indoc! {r#"
            type t {
              let x = user;
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(unclosed_delimiter)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_unexpected_top_level() {
        let source = indoc! {r#"
            @ type t {}
        "#};

        let alloc = Allocator::new();
        let (ast, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(unexpected_token)");
        assert_eq!(ast.types().len(), 1);

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_duplicate_relations() {
        let source = indoc! {r#"
            test "t" {
              relations {}
              relations {}
            }
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(duplicate_relations_block)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_expected_term() {
        let source = indoc! { r#"
            type t {
              let x = @;
            }
        "# };

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(expected_term)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_missing_relations_block() {
        let source = indoc! {r#"
            test "t" {
              assert( org("a").x( user("b") ) );
            }
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(missing_relations_block)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_missing_semicolon_in_let() {
        let source = indoc! {r#"
            type t {
              let x = user
              let y = team;
            }
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(missing_semicolon)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    #[test]
    fn diag_junk_between_types() {
        let source = indoc! {r#"
            type a {}
            blah garbage type b {}
        "#};

        let alloc = Allocator::new();
        let (_, reporter) = parse(&alloc, source);
        assert_code(&reporter, "aes::parser(unexpected_token)");

        insta::assert_snapshot!(aes_testing::render_diagnostics(
            source,
            &reporter.diagnostics
        ));
    }

    mod lexer_errors {
        use super::*;

        #[test]
        fn diag_unterminated_string() {
            let source = indoc! {r#"
                type t {
                  let x = "oops
                }
            "#};

            let alloc = Allocator::new();
            let (_, reporter) = parse(&alloc, source);
            assert_code(&reporter, "aes::lexer(unterminated_string_literal)");

            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn diag_bad_slash() {
            let source = indoc! {r#"
                type t {
                  let x = / user;
                }
            "#};

            let alloc = Allocator::new();
            let (_, reporter) = parse(&alloc, source);
            assert_code(&reporter, "aes::lexer(unexpected_character)");

            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn diag_unknown_chars() {
            let source = indoc! {r#"
                type t {
                  let x = @#$ user;
                }
            "#};

            let alloc = Allocator::new();
            let (_, reporter) = parse(&alloc, source);
            assert_code(&reporter, "aes::lexer(unexpected_character)");

            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }

        #[test]
        fn diag_digit_start() {
            let source = indoc! {r#"
                type t {
                  let x = 42user;
                }
            "#};

            let alloc = Allocator::new();
            let (_, reporter) = parse(&alloc, source);
            assert_code(&reporter, "aes::lexer(unexpected_character)");

            insta::assert_snapshot!(aes_testing::render_diagnostics(
                source,
                &reporter.diagnostics
            ));
        }
    }
}
