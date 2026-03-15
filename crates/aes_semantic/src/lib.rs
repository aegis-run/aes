use aes_allocator::Allocator;
use aes_foundation::{Diagnostic, Reporter, interner::Interner, symbols::SymbolId};

use crate::{declare::declare_schema, index::SemanticIndex, schema::Schema, verify::verify_schema};

mod declare;
mod errors;
mod index;
mod schema;
mod verify;

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
    alloc: &'src Allocator,
    source: &'src str,
    ast: &'src aes_ast::Ast<'src>,
    reporter: impl Reporter,
) -> Option<Schema<'src>> {
    let capacity = ast.types().len();
    let mut ctx = Context::new(alloc, source, capacity, reporter);

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
    source: &'src str,
    index: SemanticIndex<'src>,
    reporter: R,
}

impl<'src, R: Reporter> Context<'src, R> {
    pub fn new(alloc: &'src Allocator, source: &'src str, capacity: usize, reporter: R) -> Self {
        Self {
            source,
            index: SemanticIndex::with_capacity(alloc, capacity),
            reporter,
        }
    }

    pub(crate) fn report(&mut self, diagnostic: Diagnostic) {
        self.reporter.report(diagnostic);
    }
}
