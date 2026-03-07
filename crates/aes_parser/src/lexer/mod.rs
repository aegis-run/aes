use aes_foundation::Span;

use crate::lexer::token::{ByteClass, Token, TokenKind};

pub(crate) mod token;

#[cfg(test)]
mod tests;

pub struct Lexer<'src> {
    source: &'src [u8],
    cursor: u32,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src [u8]) -> Self {
        Self { source, cursor: 0 }
    }

    pub(crate) fn next_nontrivial(&mut self) -> Token {
        loop {
            let tok = self.next_token();
            match tok.kind() {
                TokenKind::Whitespace | TokenKind::LineComment => continue,
                _ => return tok,
            }
        }
    }

    pub fn next_token(&mut self) -> token::Token {
        if self.at_end() {
            return Token::eof(self.cursor);
        }

        let start = self.cursor;

        let kind = match token::classify(self.bump()) {
            ByteClass::Whitespace => {
                self.scan_whitespace();
                TokenKind::Whitespace
            }
            ByteClass::Ident => {
                self.scan_ident();

                let bytes = &self.source[start as usize..self.cursor as usize];
                match str::from_utf8(bytes) {
                    Ok(s) => token::classify_keyword(s),
                    Err(_) => TokenKind::Unknown,
                }
            }

            ByteClass::Slash => self.scan_line_comment().unwrap_or(TokenKind::ErrBadSlash),
            ByteClass::Quote => self
                .scan_string()
                .unwrap_or(TokenKind::ErrUnterminatedString),
            ByteClass::Colon => self.scan_colon(),

            ByteClass::Dot => TokenKind::Dot,
            ByteClass::Semicolon => TokenKind::Semicolon,
            ByteClass::LBrace => TokenKind::LBrace,
            ByteClass::RBrace => TokenKind::RBrace,
            ByteClass::LParen => TokenKind::LParen,
            ByteClass::RParen => TokenKind::RParen,
            ByteClass::Pipe => TokenKind::Pipe,
            ByteClass::Amp => TokenKind::Amp,
            ByteClass::Minus => TokenKind::Minus,
            ByteClass::Eq => TokenKind::Eq,

            ByteClass::Digit | ByteClass::Other => TokenKind::Unknown,
        };

        Token::new(kind, Span::from_range(start, self.cursor))
    }

    #[inline]
    fn scan_whitespace(&mut self) {
        self.eat_while(|it| token::classify(it) == ByteClass::Whitespace);
    }

    #[inline]
    fn scan_ident(&mut self) {
        self.eat_while(|it| {
            let c = token::classify(it);
            c == ByteClass::Ident || c == ByteClass::Digit
        });
    }

    #[inline]
    fn scan_line_comment(&mut self) -> Option<TokenKind> {
        if !self.eat_if(|it| it == b'/') {
            return None;
        }

        self.eat_while(|b| b != b'\n');
        Some(TokenKind::LineComment)
    }

    #[inline]
    fn scan_string(&mut self) -> Option<TokenKind> {
        loop {
            if self.at_end() {
                return None;
            }
            if self.bump() == b'"' {
                return Some(TokenKind::String);
            }
        }
    }

    #[inline]
    fn scan_colon(&mut self) -> TokenKind {
        if self.eat_if(|it| it == b':') {
            TokenKind::ColonColon
        } else {
            TokenKind::Colon
        }
    }

    #[inline(always)]
    fn at_end(&self) -> bool {
        self.cursor as usize >= self.source.len()
    }

    #[inline(always)]
    fn current(&self) -> u8 {
        self.source[self.cursor as usize]
    }

    #[inline(always)]
    fn bump(&mut self) -> u8 {
        let b = self.current();
        self.cursor += 1;
        b
    }

    #[inline(always)]
    fn eat_if(&mut self, pred: impl Fn(u8) -> bool) -> bool {
        if self.at_end() || !pred(self.current()) {
            return false;
        }

        self.cursor += 1;
        true
    }

    #[inline]
    fn eat_while(&mut self, pred: impl Fn(u8) -> bool) -> u32 {
        let start = self.cursor;

        while !self.at_end() && pred(self.current()) {
            self.cursor += 1;
        }

        self.cursor - start
    }
}
