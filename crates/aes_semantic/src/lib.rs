//! Semantic analysis and verification for the Aegis Schema Language.
//!
//! `aes_semantic` takes a raw AST from `aes_parser` and performs a **Two-Pass Analysis**
//! to resolve references, ensure type safety, and output an optimized read-only [`Schema`].
//!
//! ### The Two-Pass Architecture
//! 1. **Declaration Pass** ([`declare`]): Walks the AST to extract all types, relations, and permissions.
//!    It assigns each unique string a fast `SymbolId` via `aes_foundation::interner` and catches basic
//!    name collisions (e.g., duplicate types or overlapping relations/permissions).
//! 2. **Verification Pass** ([`verify`]): Re-walks the AST using the accumulated definitions. It resolves
//!    all `.relation` and `.permission` references and enforces language-level rules (e.g., no self-references
//!    in `let` blocks, no traversals over computed `def` permissions).
use aes_foundation::{Diagnostic, Reporter, interner::Interner, symbols::SymbolId, vfs::FileRef};

use crate::{declare::declare_schema, index::SemanticIndex, verify::verify_schema};

mod declare;
mod errors;
mod index;
mod schema;
mod verify;

pub use schema::*;

#[derive(Debug, Clone, Copy)]
pub struct TypeMarker;
pub type TypeSymbol = SymbolId<TypeMarker>;
pub(crate) type TypeInterner<'src> = Interner<'src, TypeMarker>;

#[derive(Debug, Clone, Copy)]
pub struct RelationMarker;
pub type RelationSymbol = SymbolId<RelationMarker>;
pub(crate) type RelationInterner<'src> = Interner<'src, RelationMarker>;

#[derive(Debug, Clone, Copy)]
pub struct PermissionMarker;
pub type PermissionSymbol = SymbolId<PermissionMarker>;
pub(crate) type PermissionInterner<'src> = Interner<'src, PermissionMarker>;

pub fn analyze<'src>(
    file: FileRef<'src>,
    ast: &aes_ast::Ast<'src>,
    reporter: impl Reporter,
) -> Option<Schema<'src>> {
    let capacity = ast.types().len();
    let mut ctx = Context::new(file, capacity, reporter);

    declare_schema(&mut ctx, ast);
    if ctx.reporter.has_errors() {
        return None;
    }

    verify_schema(&mut ctx, ast);
    if ctx.reporter.has_errors() {
        return None;
    }

    Some(ctx.index.into_schema())
}

pub(crate) struct Context<'src, R: Reporter> {
    file: FileRef<'src>,
    index: SemanticIndex<'src>,
    reporter: R,
}

impl<'src, R: Reporter> Context<'src, R> {
    pub fn new(file: FileRef<'src>, capacity: usize, reporter: R) -> Self {
        Self {
            file,
            index: SemanticIndex::with_capacity(file.alloc(), capacity),
            reporter,
        }
    }

    pub(crate) fn report(&mut self, diagnostic: Diagnostic) {
        self.reporter.report(diagnostic);
    }
}
