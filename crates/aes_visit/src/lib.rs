//! AST traversal via the visitor pattern.
//!
//! # Usage
//!
//! Implement [`Visitor`] and override the methods you care about:
//!
//! ```
//! # use aes_visit::{Visitor, walk_type_def};
//! # use aes_ast::{Ast, TypeDefId};
//! struct MyVisitor<'a> {
//!     ast: &'a Ast<'a>,
//! }
//!
//! impl<'a> Visitor<'a> for MyVisitor<'a> {
//!     fn ast(&self) -> &Ast<'a> { self.ast }
//!
//!     fn type_def(&mut self, id: TypeDefId) {
//!         // Custom logic before walking children
//!         walk_type_def(self, id);  // Recurse into children
//!         // Custom logic after walking children
//!     }
//! }
//! ```
//!
//! # Structure
//!
//! - [`schema()`] - Entry point; visits all types and tests
//! - [`Visitor`] - Trait with default `walk_*` implementations
//! - Leaf methods (`expr_*`, `relation`, `assert`) - Override for specific node types

mod expr;
mod test_def;
#[cfg(test)]
mod tests;
mod type_def;

use aes_ast::{
    AssertId, Ast, DefMemberId, ExprId, ExprTermBinary, ExprTermParen, ExprTermSelfRef,
    ExprTermTraversal, ExprTermTypeRef, ExprTermUsersetTypeRef, LetMemberId, RelationId, TestDefId,
    TypeDefId,
};

pub use expr::*;
pub use test_def::*;
pub use type_def::*;

pub fn schema<'src>(visit: &mut impl Visitor<'src>) {
    let type_count = visit.ast().types().len();
    for index in 0..type_count {
        visit.type_def(TypeDefId::new(index as u32));
    }

    let test_count = visit.ast().tests().len();
    for index in 0..test_count {
        visit.test_def(TestDefId::new(index as u32));
    }
}

pub trait Visitor<'src>: Sized {
    fn ast(&self) -> &Ast<'src>;

    fn type_def(&mut self, id: TypeDefId) {
        type_def::walk_type_def(self, id);
    }

    fn let_member(&mut self, id: LetMemberId) {
        type_def::walk_let_member(self, id);
    }

    fn def_member(&mut self, id: DefMemberId) {
        type_def::walk_def_member(self, id);
    }

    fn expr(&mut self, id: ExprId) {
        expr::walk_expr(self, id);
    }

    fn expr_paren(&mut self, _id: ExprId, expr: ExprTermParen) {
        expr::walk_expr(self, expr.inner);
    }

    #[allow(unused_variables)]
    fn expr_self_ref(&mut self, id: ExprId, expr: ExprTermSelfRef) {}

    #[allow(unused_variables)]
    fn expr_traversal(&mut self, id: ExprId, expr: ExprTermTraversal) {}

    #[allow(unused_variables)]
    fn expr_type_ref(&mut self, id: ExprId, expr: ExprTermTypeRef) {}

    #[allow(unused_variables)]
    fn expr_userset_type_ref(&mut self, id: ExprId, expr: ExprTermUsersetTypeRef) {}

    #[allow(unused_variables)]
    fn expr_binary(&mut self, id: ExprId, expr: ExprTermBinary) {}

    fn test_def(&mut self, id: TestDefId) {
        test_def::walk_test_def(self, id);
    }

    #[allow(unused_variables)]
    fn relation(&mut self, id: RelationId) {}

    #[allow(unused_variables)]
    fn assert(&mut self, id: AssertId) {}
}
