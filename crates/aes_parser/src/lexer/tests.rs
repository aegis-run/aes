use crate::lexer::{
    Lexer,
    token::{self, ByteClass, TokenKind},
};

fn lex_all(source: &str) -> Vec<(TokenKind, &str)> {
    let mut lexer = Lexer::new(source.as_bytes());
    std::iter::from_fn(|| Some(lexer.next_token()))
        .take_while(|tok| tok.kind() != TokenKind::Eof)
        .map(|tok| (tok.kind(), tok.span().text(source)))
        .collect()
}

fn lex_nontrivial(source: &str) -> Vec<(TokenKind, &str)> {
    let mut lexer = Lexer::new(source.as_bytes());

    std::iter::from_fn(|| Some(lexer.next_nontrivial()))
        .take_while(|tok| tok.kind() != TokenKind::Eof)
        .map(|tok| (tok.kind(), tok.span().text(source)))
        .collect()
}

fn kinds(tokens: &[(TokenKind, &str)]) -> Vec<TokenKind> {
    tokens.iter().map(|(k, _)| *k).collect()
}

fn texts<'a>(tokens: &'a [(TokenKind, &'a str)]) -> Vec<&'a str> {
    tokens.iter().map(|(_, t)| *t).collect()
}

mod eof {
    use super::*;

    #[test]
    fn empty_input_yields_eof() {
        let mut lexer = Lexer::new(b"");
        let tok = lexer.next_token();
        assert_eq!(tok.kind(), TokenKind::Eof);
        assert_eq!(tok.span().start(), 0);
        assert_eq!(tok.span().size(), 0);
    }

    #[test]
    fn repeated_eof_is_stable() {
        let mut lexer = Lexer::new(b"");
        for _ in 0..3 {
            let tok = lexer.next_token();
            assert_eq!(tok.kind(), TokenKind::Eof);
        }
    }
}

mod keywords {
    use super::*;

    #[test]
    fn all_keywords() {
        let toks = lex_nontrivial("type let def test relations assert assert_not");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwType,
                TokenKind::KwLet,
                TokenKind::KwDef,
                TokenKind::KwTest,
                TokenKind::KwRelations,
                TokenKind::KwAssert,
                TokenKind::KwAssertNot
            ]
        );
    }

    #[test]
    fn prefix_is_ident() {
        let toks = lex_nontrivial("types letting define testing related asserted");
        assert!(kinds(&toks).iter().all(|k| *k == TokenKind::Ident));
    }

    #[test]
    fn suffix_is_ident() {
        let toks = lex_nontrivial("mytype xlet _def");
        assert!(kinds(&toks).iter().all(|k| *k == TokenKind::Ident));
    }

    #[test]
    fn case_sensitive() {
        let toks = lex_nontrivial("Type LET DEF Test RELATIONS Assert ASSERT_NOT");
        assert!(kinds(&toks).iter().all(|k| *k == TokenKind::Ident));
    }
}

mod identifiers {
    use super::*;

    #[test]
    fn simple() {
        let toks = lex_nontrivial("foo");
        assert_eq!(kinds(&toks), vec![TokenKind::Ident]);
        assert_eq!(texts(&toks), vec!["foo"]);
    }

    #[test]
    fn with_digits() {
        let toks = lex_nontrivial("user42 item_3");
        assert_eq!(kinds(&toks), vec![TokenKind::Ident, TokenKind::Ident]);
        assert_eq!(texts(&toks), vec!["user42", "item_3"]);
    }

    #[test]
    fn with_underscores() {
        let toks = lex_nontrivial("_private __double snake_case a_b_c_d");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Ident
            ]
        );
    }

    #[test]
    fn single_letter() {
        let toks = lex_nontrivial("a B _");
        assert_eq!(
            kinds(&toks),
            vec![TokenKind::Ident, TokenKind::Ident, TokenKind::Ident]
        );
    }
}

mod strings {
    use super::*;

    #[test]
    fn simple() {
        let toks = lex_nontrivial(r#""alice""#);
        assert_eq!(kinds(&toks), vec![TokenKind::String]);
        assert_eq!(texts(&toks), vec![r#""alice""#]);
    }

    #[test]
    fn empty() {
        let toks = lex_nontrivial(r#""""#);
        assert_eq!(kinds(&toks), vec![TokenKind::String]);
        assert_eq!(texts(&toks), vec![r#""""#]);
    }

    #[test]
    fn with_spaces() {
        let toks = lex_nontrivial(r#""hello world""#);
        assert_eq!(kinds(&toks), vec![TokenKind::String]);
    }

    #[test]
    fn unterminated() {
        let toks = lex_nontrivial(r#""unterminated"#);
        assert_eq!(kinds(&toks), vec![TokenKind::ErrUnterminatedString]);
    }

    #[test]
    fn unterminated_span_covers_rest() {
        let source = r#""oops"#;
        let toks = lex_all(source);
        assert_eq!(kinds(&toks), vec![TokenKind::ErrUnterminatedString]);
        assert_eq!(texts(&toks), vec![r#""oops"#]);
    }
}

mod punctuation {
    use super::*;

    #[test]
    fn all_with_spaces() {
        let toks = lex_nontrivial(". ; { } ( ) | & - =");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Dot,
                TokenKind::Semicolon,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Pipe,
                TokenKind::Amp,
                TokenKind::Minus,
                TokenKind::Eq
            ]
        );
    }

    #[test]
    fn all_no_spaces() {
        let toks = lex_nontrivial(".;{}()|&-=");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Dot,
                TokenKind::Semicolon,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Pipe,
                TokenKind::Amp,
                TokenKind::Minus,
                TokenKind::Eq
            ]
        );
    }
}

mod colons {
    use super::*;

    #[test]
    fn single() {
        let toks = lex_nontrivial(":");
        assert_eq!(kinds(&toks), vec![TokenKind::Colon]);
    }

    #[test]
    fn double() {
        let toks = lex_nontrivial("::");
        assert_eq!(kinds(&toks), vec![TokenKind::ColonColon]);
    }

    #[test]
    fn then_ident() {
        let toks = lex_nontrivial(":foo");
        assert_eq!(kinds(&toks), vec![TokenKind::Colon, TokenKind::Ident]);
    }

    #[test]
    fn double_then_ident() {
        let toks = lex_nontrivial("::member");
        assert_eq!(kinds(&toks), vec![TokenKind::ColonColon, TokenKind::Ident]);
    }

    #[test]
    fn triple() {
        let toks = lex_nontrivial(":::");
        assert_eq!(kinds(&toks), vec![TokenKind::ColonColon, TokenKind::Colon]);
    }
}

mod comments {
    use super::*;

    #[test]
    fn skipped_as_trivia() {
        let toks = lex_nontrivial("foo // this is a comment\nbar");
        assert_eq!(kinds(&toks), vec![TokenKind::Ident, TokenKind::Ident]);
        assert_eq!(texts(&toks), vec!["foo", "bar"]);
    }

    #[test]
    fn at_eof() {
        let toks = lex_nontrivial("foo // trailing");
        assert_eq!(kinds(&toks), vec![TokenKind::Ident]);
    }

    #[test]
    fn only_comment() {
        let toks = lex_nontrivial("// just a comment");
        assert!(toks.is_empty());
    }

    #[test]
    fn preserved_in_raw_next_token() {
        let toks = lex_all("// hello");
        assert_eq!(kinds(&toks), vec![TokenKind::LineComment]);
    }

    #[test]
    fn lone_slash_is_error() {
        let toks = lex_all("/");
        assert_eq!(kinds(&toks), vec![TokenKind::ErrBadSlash]);
    }

    #[test]
    fn slash_before_non_slash() {
        let toks = lex_all("/x");
        assert_eq!(kinds(&toks), vec![TokenKind::ErrBadSlash, TokenKind::Ident]);
    }
}

mod whitespace {
    use super::*;

    #[test]
    fn coalesced() {
        let toks = lex_all("   \t\t\n\r\n  ");
        assert_eq!(kinds(&toks), vec![TokenKind::Whitespace]);
    }

    #[test]
    fn split_by_non_ws() {
        let toks = lex_all("  x  ");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Whitespace,
                TokenKind::Ident,
                TokenKind::Whitespace
            ]
        );
    }
}

mod unknown {
    use super::*;

    #[test]
    fn digit_alone() {
        let toks = lex_all("42");
        assert_eq!(kinds(&toks), vec![TokenKind::Unknown, TokenKind::Unknown]);
    }

    #[test]
    fn special_chars() {
        let toks = lex_all("@#$%^");
        assert!(kinds(&toks).iter().all(|k| *k == TokenKind::Unknown));
    }
}

mod error_surfacing {
    use super::*;

    #[test]
    fn nontrivial_returns_bad_slash() {
        let toks = lex_nontrivial(r#"foo / bar"#);
        assert_eq!(
            kinds(&toks),
            vec![TokenKind::Ident, TokenKind::ErrBadSlash, TokenKind::Ident]
        );
    }

    #[test]
    fn nontrivial_returns_unterminated_string() {
        let toks = lex_nontrivial(r#"test "oops"#);
        assert_eq!(
            kinds(&toks),
            vec![TokenKind::KwTest, TokenKind::ErrUnterminatedString]
        );
    }
}

mod realistic {
    use super::*;

    #[test]
    fn type_definition() {
        let toks = lex_nontrivial("type user {}");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwType,
                TokenKind::Ident,
                TokenKind::LBrace,
                TokenKind::RBrace
            ]
        );
    }

    #[test]
    fn member_let() {
        let toks = lex_nontrivial("let parent = organization | team;");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Ident,
                TokenKind::Semicolon
            ]
        );
    }

    #[test]
    fn member_def_with_self_ref() {
        let toks = lex_nontrivial("def member = .maintainer | .direct_member;");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwDef,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Semicolon
            ]
        );
    }

    #[test]
    fn def_with_traversal() {
        let toks = lex_nontrivial("def push = .writer | .organization.owner;");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwDef,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Semicolon
            ]
        );
    }

    #[test]
    fn userset_type_ref() {
        let toks = lex_nontrivial("let reader = user | team::member;");
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Ident,
                TokenKind::ColonColon,
                TokenKind::Ident,
                TokenKind::Semicolon
            ]
        );
    }

    #[test]
    fn relation_block() {
        let source = r#"organization("acmecorp") .{
      .owner: user("alice");
    };"#;
        let toks = lex_nontrivial(source);
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::String,
                TokenKind::RParen,
                TokenKind::Dot,
                TokenKind::LBrace,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Colon,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::String,
                TokenKind::RParen,
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Semicolon,
            ]
        );
    }

    #[test]
    fn inline_relation() {
        let source = r#"team("api_team") .parent: team("infrastructure");"#;
        let toks = lex_nontrivial(source);
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::String,
                TokenKind::RParen,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Colon,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::String,
                TokenKind::RParen,
                TokenKind::Semicolon,
            ]
        );
    }

    #[test]
    fn assertion() {
        let source = r#"assert( organization("acmecorp").manage_billing( user("alice") ) );"#;
        let toks = lex_nontrivial(source);
        assert_eq!(
            kinds(&toks),
            vec![
                TokenKind::KwAssert,
                TokenKind::LParen,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::String,
                TokenKind::RParen,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::String,
                TokenKind::RParen,
                TokenKind::RParen,
                TokenKind::RParen,
                TokenKind::Semicolon,
            ]
        );
    }

    #[test]
    fn assert_not() {
        let source = r#"assert_not( repository("x").push( user("eve") ) );"#;
        let toks = lex_nontrivial(source);
        assert_eq!(kinds(&toks)[0], TokenKind::KwAssertNot);
    }

    #[test]
    fn full_type_no_errors() {
        let source = r#"type repository {
    let organization = organization;
    let reader = user | team::member;
    def push = .writer | .organization.owner;
    def read = .clone | .organization.owner;
}"#;
        let toks = lex_nontrivial(source);
        assert!(
            toks.iter().all(|(k, _)| !k.is_error()),
            "found error token in valid type def"
        );
        assert_eq!(
            kinds(&toks),
            vec![
                // type repository {
                TokenKind::KwType,
                TokenKind::Ident,
                TokenKind::LBrace,
                // let organization = organization;
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Ident,
                TokenKind::Semicolon,
                // let reader = user | team::member;
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Ident,
                TokenKind::ColonColon,
                TokenKind::Ident,
                TokenKind::Semicolon,
                // def push = .writer | .organization.owner;
                TokenKind::KwDef,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Semicolon,
                // def read = .clone | .organization.owner;
                TokenKind::KwDef,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Pipe,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Semicolon,
                // }
                TokenKind::RBrace,
            ]
        );
    }
}

mod lut {
    use super::*;

    #[test]
    fn all_lowercase_are_ident() {
        for b in b'a'..=b'z' {
            assert_eq!(token::classify(b), ByteClass::Ident);
        }
    }

    #[test]
    fn all_uppercase_are_ident() {
        for b in b'A'..=b'Z' {
            assert_eq!(token::classify(b), ByteClass::Ident);
        }
    }

    #[test]
    fn all_digits_are_digit() {
        for b in b'0'..=b'9' {
            assert_eq!(token::classify(b), ByteClass::Digit);
        }
    }

    #[test]
    fn underscore_is_ident() {
        assert_eq!(token::classify(b'_'), ByteClass::Ident);
    }

    #[test]
    fn null_byte_is_other() {
        assert_eq!(token::classify(0), ByteClass::Other);
    }

    #[test]
    fn high_bytes_are_other() {
        for b in 128u8..=255 {
            assert_eq!(token::classify(b), ByteClass::Other);
        }
    }
}

mod properties {
    use super::*;
    use proptest::prelude::*;

    fn lex(input: &[u8]) -> Vec<token::Token> {
        let mut lexer = Lexer::new(input);
        let mut tokens = vec![];

        loop {
            let tok = lexer.next_token();
            tokens.push(tok);

            if tok.kind() == TokenKind::Eof {
                break;
            }
        }

        tokens
    }

    proptest! {
        #[test]
        fn never_panics(ref input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = lex(input);
        }

        #[test]
        fn spans_in_bounds(ref input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let tokens = lex(input);
            for tok in &tokens {
                prop_assert!((tok.span().start() as usize) <= input.len());
                prop_assert!((tok.span().end() as usize) <= input.len());
            }
        }

        #[test]
        fn spans_contiguous_and_cover_input(ref input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let tokens = lex(input);
            let non_eof: Vec<_> = tokens.iter().filter(|t| t.kind() != TokenKind::Eof).collect();

            let mut prev_end = 0u32;
            for tok in &non_eof {
                prop_assert_eq!(tok.span().start(), prev_end);
                prev_end = tok.span().end();
            }

            prop_assert_eq!(prev_end as usize, input.len());
        }

        #[test]
        fn no_zero_size_tokens_except_eof(ref input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let tokens = lex(input);
            for tok in &tokens {
                if tok.kind() != TokenKind::Eof {
                    prop_assert!(tok.span().size() > 0);
                }
            }
        }

        #[test]
        fn eof_is_last_and_emitted_once(ref input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let tokens = lex(input);
            prop_assert!(!tokens.is_empty());
            prop_assert_eq!(tokens.last().unwrap().kind(), TokenKind::Eof);

            let eof_count = tokens.iter().filter(|t| t.kind() == TokenKind::Eof).count();
            prop_assert_eq!(eof_count, 1, "Eof emitted {} times", eof_count);
        }

        #[test]
        fn text_roundtrip(ref input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let tokens = lex(input);
            let non_eof = tokens.iter().filter(|t| t.kind() != TokenKind::Eof);

            let mut reconstructed = Vec::new();
            for tok in non_eof {
              let start = tok.span().start();
              let end = tok.span().end();
                reconstructed.extend_from_slice(&input[start as usize..end as usize]);
            }

            prop_assert_eq!(&reconstructed, input, "roundtrip mismatch");
        }
    }
}
