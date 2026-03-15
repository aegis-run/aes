use aes_foundation::Span;

#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Token {
    span: Span,
    kind: TokenKind,
}

aes_foundation::const_assert!(std::mem::size_of::<Token>() == 12);

impl Token {
    #[inline]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { span, kind }
    }

    #[inline]
    pub const fn eof(at: u32) -> Self {
        Self::new(TokenKind::Eof, Span::empty(at))
    }

    #[inline]
    pub const fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub const fn kind(&self) -> TokenKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenKind {
    Ident,
    String,

    KwType,
    KwLet,
    KwDef,
    KwTest,
    KwRelations,
    KwAssert,
    KwAssertNot,

    Dot,
    Colon,
    ColonColon,
    Semicolon,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Pipe,
    Amp,
    Minus,
    Eq,

    Whitespace,
    LineComment,

    Eof,

    Unknown,
    ErrUnterminatedString,
    ErrBadSlash,
}

impl TokenKind {
    pub const fn is_error(self) -> bool {
        matches!(
            self,
            TokenKind::Unknown | TokenKind::ErrUnterminatedString | TokenKind::ErrBadSlash
        )
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TokenKind::Ident => "identifier",
            TokenKind::String => "string",
            TokenKind::KwType => "'type'",
            TokenKind::KwLet => "'let'",
            TokenKind::KwDef => "'def'",
            TokenKind::KwTest => "'test'",
            TokenKind::KwRelations => "'relations'",
            TokenKind::KwAssert => "'assert'",
            TokenKind::KwAssertNot => "'assert_not'",
            TokenKind::Dot => "'.'",
            TokenKind::Colon => "':'",
            TokenKind::ColonColon => "'::'",
            TokenKind::Semicolon => "';'",
            TokenKind::LBrace => "'{'",
            TokenKind::RBrace => "'}'",
            TokenKind::LParen => "'('",
            TokenKind::RParen => "')'",
            TokenKind::Pipe => "'|'",
            TokenKind::Amp => "'&'",
            TokenKind::Minus => "'-'",
            TokenKind::Eq => "'='",
            TokenKind::Whitespace => "whitespace",
            TokenKind::LineComment => "comment",
            TokenKind::Eof => "end of file",
            TokenKind::Unknown => "unknown character",
            TokenKind::ErrUnterminatedString => "unterminated string",
            TokenKind::ErrBadSlash => "unexpected '/'",
        })
    }
}

#[inline(always)]
pub(crate) fn classify(b: u8) -> ByteClass {
    BYTE_CLASS[b as usize]
}

#[inline(always)]
pub(crate) fn classify_keyword(s: &str) -> TokenKind {
    match s {
        "type" => TokenKind::KwType,
        "let" => TokenKind::KwLet,
        "def" => TokenKind::KwDef,
        "test" => TokenKind::KwTest,
        "relations" => TokenKind::KwRelations,
        "assert" => TokenKind::KwAssert,
        "assert_not" => TokenKind::KwAssertNot,
        _ => TokenKind::Ident,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ByteClass {
    Ident,
    Digit,
    Whitespace,
    Slash,
    Quote,
    Dot,
    Colon,
    Semicolon,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Pipe,
    Amp,
    Minus,
    Eq,
    Other,
}

const BYTE_CLASS: [ByteClass; 256] = {
    use ByteClass::*;
    let mut t = [Other; 256];

    let mut c = b'a';
    while c <= b'z' {
        t[c as usize] = Ident;
        c += 1;
    }

    let mut c = b'A';
    while c <= b'Z' {
        t[c as usize] = Ident;
        c += 1;
    }

    let mut c = b'0';
    while c <= b'9' {
        t[c as usize] = Digit;
        c += 1;
    }

    t[b'_' as usize] = Ident;
    t[b' ' as usize] = Whitespace;
    t[b'\t' as usize] = Whitespace;
    t[b'\n' as usize] = Whitespace;
    t[b'\r' as usize] = Whitespace;
    t[b'/' as usize] = Slash;
    t[b'"' as usize] = Quote;
    t[b'.' as usize] = Dot;
    t[b':' as usize] = Colon;
    t[b';' as usize] = Semicolon;
    t[b'{' as usize] = LBrace;
    t[b'}' as usize] = RBrace;
    t[b'(' as usize] = LParen;
    t[b')' as usize] = RParen;
    t[b'|' as usize] = Pipe;
    t[b'&' as usize] = Amp;
    t[b'-' as usize] = Minus;
    t[b'=' as usize] = Eq;

    t
};
