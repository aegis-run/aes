//! Transforming raw UTF-8 bytes into a structured Abstract Syntax Tree (`aes_ast`).
//!
//! `aes_parser` provides a modular, zero-copy Lexer and a Recursive Descent Parser with
//! built-in Pratt Parsing for expressions. The parser relies heavily on `aes_foundation`
//! for memory-pool allocation (`Id`, `Range`) and error tracking (`Diagnostic`).
mod errors;
mod lexer;
mod parser;

pub(crate) use lexer::*;
pub use parser::*;
