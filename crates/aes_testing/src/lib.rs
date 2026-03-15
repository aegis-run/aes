pub mod ast;
mod diagnostic;
mod reporter;

pub use diagnostic::*;
pub use reporter::*;

pub const SPAN: aes_foundation::Span = aes_foundation::Span::from_range(0, 0);
