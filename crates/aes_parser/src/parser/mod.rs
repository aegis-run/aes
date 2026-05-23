use aes_ast::{Ast, AstBuilder};
use aes_foundation::{Diagnostic, Reporter, Span, vfs::FileRef};

use crate::{
    Lexer, errors,
    token::{Token, TokenKind},
};

mod expr;
mod test_def;
#[cfg(test)]
mod tests;
mod type_def;

/// A Recursive Descent parser that constructs an `Ast` from a stream of `Token`s.
///
/// Under the hood, the parser uses an [`AstBuilder`] to push AST nodes into contiguous
/// Arena memory pools. Because nodes are referenced via `u32` indices (`Id<T>`), the parser
/// never deals with complex Rust lifetimes or scattered `Box` allocations.
///
/// It also implements robust error recovery out of the box: when parsing fails, it buffers
/// a `Diagnostic` and synchronizes to the next known-good token (e.g. `}` or `;`) before continuing.
pub struct Parser<'src, R: Reporter> {
    lexer: Lexer<'src>,
    ast: AstBuilder<'src>,

    prev: Span,
    token: Token,

    reporter: R,
}

impl<'src, R: Reporter> Parser<'src, R> {
    pub fn new(file: FileRef<'src>, reporter: R) -> Self {
        let mut lexer = Lexer::new(file.source().as_bytes());

        let token = lexer.next_nontrivial();

        Self {
            lexer,
            ast: AstBuilder::new(file.alloc()),
            prev: Span::empty(token.span().start()),
            token,
            reporter,
        }
    }

    pub fn parse(mut self) -> Ast<'src> {
        while !self.at(TokenKind::Eof) {
            match self.token.kind() {
                TokenKind::KwType => self.type_def(),
                TokenKind::KwTest => self.test_def(),
                _ => {
                    self.report(errors::unexpected_token(self.token));
                    self.skip_while(|k| !matches!(k, TokenKind::KwType | TokenKind::KwTest));
                }
            }
        }

        self.ast.finish()
    }

    pub(crate) fn report(&mut self, diagnostic: Diagnostic) {
        self.reporter.report(diagnostic);
    }
}

impl<'src, R: Reporter> Parser<'src, R> {
    fn start_span(&self) -> u32 {
        self.token.span().start()
    }

    fn end_span(&self, start: u32) -> Span {
        Span::from_range(start, self.prev.end())
    }

    fn advance(&mut self) -> Token {
        let prev = self.token;
        self.prev = prev.span();
        self.token = self.next_meaningful();
        prev
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.token.kind() == kind
    }

    fn expect(&mut self, kind: TokenKind) -> Span {
        if self.at(kind) {
            return self.advance().span();
        }

        self.report(errors::expected(kind, self.token));
        Span::empty(self.token.span().start())
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if !self.at(kind) {
            return false;
        }
        self.skip();
        true
    }

    fn skip(&mut self) {
        let _ = self.advance();
    }

    fn skip_while(&mut self, f: impl Fn(TokenKind) -> bool) {
        while !self.at(TokenKind::Eof) && f(self.token.kind()) {
            self.skip();
        }
    }

    #[must_use]
    fn ident(&mut self) -> Span {
        self.expect(TokenKind::Ident)
    }

    #[must_use]
    fn string(&mut self) -> Span {
        self.expect(TokenKind::String)
    }

    fn braced<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let open = self.expect(TokenKind::LBrace);
        let result = f(self);

        if !self.at(TokenKind::RBrace) {
            self.report(errors::unclosed_delimiter(open, "'{'", self.token));
        } else {
            self.skip();
        }

        result
    }

    fn parenthesized<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let open = self.expect(TokenKind::LParen);
        let result = f(self);

        if !self.at(TokenKind::RParen) {
            self.report(errors::unclosed_delimiter(open, "'('", self.token));
        } else {
            self.skip();
        }

        result
    }

    fn semicolon(&mut self) {
        if self.at(TokenKind::Semicolon) {
            self.skip();
        } else {
            self.report(errors::missing_semicolon(self.prev));
        }
    }

    fn next_meaningful(&mut self) -> Token {
        loop {
            let token = self.lexer.next_nontrivial();
            if token.kind().is_error() {
                self.report(errors::from_lexer_error(token));
            } else {
                return token;
            }
        }
    }
}
