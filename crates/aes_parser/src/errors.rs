use aes_foundation::{Diagnostic, Span};

use crate::lexer::token::{Token, TokenKind};

pub fn unexpected_character(span: Span) -> Diagnostic {
    Diagnostic::error("unexpected character")
        .with_code("aes::lexer", "unexpected_character")
        .with_label(span.label("not valid here"))
}

pub fn unterminated_str_literal(span: Span) -> Diagnostic {
    Diagnostic::error("unterminated string literal")
        .with_code("aes::lexer", "unterminated_string_literal")
        .with_label(span.label("string starts here"))
        .with_help("add a closing `\"` to terminate the string")
}

pub fn expected(expected: TokenKind, found: Token) -> Diagnostic {
    Diagnostic::error(format!("expected {expected}, found {}", found.kind()))
        .with_code("aes::parser", "expected_token")
        .with_label(found.span().label(format!("expected {expected}")))
}

pub fn unexpected_token(found: Token) -> Diagnostic {
    Diagnostic::error(format!("unexpected {}", found.kind()))
        .with_code("aes::parser", "unexpected_token")
        .with_label(found.span().label("not expected here"))
}

pub fn expected_term(found: Token) -> Diagnostic {
    Diagnostic::error(format!("expected expression term, found {}", found.kind()))
        .with_code("aes::parser", "expected_term")
        .with_label(
            found
                .span()
                .label("expected `.relation`, `type`, `type::member`, or `(`"),
        )
}

pub fn assert_before_relations(found: Token) -> Diagnostic {
    Diagnostic::error("assertion found before relations block")
        .with_code("aes::parser", "assert_before_relations")
        .with_label(
            found
                .span()
                .label("this assertion must come after the relations block"),
        )
        .with_help("move the `relations { ... }` block before any `assert` statements")
}

pub fn duplicate_relations_block(found: Token) -> Diagnostic {
    Diagnostic::error("duplicate relations block")
        .with_code("aes::parser", "duplicate_relations_block")
        .with_label(
            found
                .span()
                .label("only one relations block is allowed per test"),
        )
        .with_help("merge all relation statements into a single `relations { ... }` block")
}

pub fn missing_semicolon(after: Span) -> Diagnostic {
    Diagnostic::error("missing semicolon")
        .with_code("aes::parser", "missing_semicolon")
        .with_label(after.label("expected ';' after this"))
}

pub fn unclosed_delimiter(open: Span, open_char: &str, found: Token) -> Diagnostic {
    Diagnostic::error(format!("unclosed {open_char}"))
        .with_code("aes::parser", "unclosed_delimiter")
        .with_label(open.label(format!("opening {open_char} here")))
        .and_label(
            found
                .span()
                .label(format!("expected matching close for {open_char}")),
        )
}

pub fn missing_relations_block(found: Token) -> Diagnostic {
    Diagnostic::error(format!(
        "expected 'relations' block, found {}",
        found.kind()
    ))
    .with_code("aes::parser", "missing_relations_block")
    .with_label(found.span().label("expected 'relations' keyword here"))
    .with_help("a test must begin with `relations { ... }` before any assertions")
}

pub fn expected_relation_name_or_block(found: Token) -> Diagnostic {
    Diagnostic::error(format!(
        "expected a relation name or `{{` after `.`, found {}",
        found.kind()
    ))
    .with_code("aes::parser", "expected_relation_name_or_block")
    .with_label(found.span().label("expected relation name or block here"))
    .with_help("use `instance.relation: value;` or `instance .{ .relation: value; };`")
}

pub fn expected_permission_after_colons(found: Token) -> Diagnostic {
    Diagnostic::error(format!(
        "expected a permission name after `::`, found {}",
        found.kind()
    ))
    .with_code("aes::parser", "expected_permission_after_colons")
    .with_label(found.span().label("expected permission name here"))
    .with_help("example: `team(\"infra\")::member`")
}

pub fn expected_type_name(found: Token) -> Diagnostic {
    Diagnostic::error(format!("expected a type name, found {}", found.kind()))
        .with_code("aes::parser", "expected_type_name")
        .with_label(found.span().label("expected type name here"))
        .with_help("example: `team(\"infra\")`")
}

pub fn from_lexer_error(token: Token) -> Diagnostic {
    match token.kind() {
        TokenKind::ErrUnterminatedString => unterminated_str_literal(token.span()),
        TokenKind::ErrBadSlash => unexpected_character(token.span()),
        TokenKind::Unknown => unexpected_character(token.span()),
        _ => unexpected_character(token.span()),
    }
}
