pub mod ast;
mod diagnostic;
pub mod file;
pub mod generate;
mod reporter;

pub use diagnostic::*;
pub use file::file_ref;
pub use reporter::*;

pub const SPAN: aes_foundation::Span = aes_foundation::Span::from_range(0, 0);
