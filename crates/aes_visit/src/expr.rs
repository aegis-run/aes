use aes_ast::{ExprId, ExprTerm};

use crate::Visitor;

pub fn walk_expr<'src>(visit: &mut impl Visitor<'src>, id: ExprId) {
    match visit.ast().exprs().at(id).term() {
        ExprTerm::Paren(expr) => visit.expr_paren(id, expr),
        ExprTerm::SelfRef(expr) => visit.expr_self_ref(id, expr),
        ExprTerm::Traversal(expr) => visit.expr_traversal(id, expr),
        ExprTerm::TypeRef(expr) => visit.expr_type_ref(id, expr),
        ExprTerm::UsersetTypeRef(expr) => visit.expr_userset_type_ref(id, expr),
        ExprTerm::Binary(expr) => {
            visit.expr_binary(id, expr);
            visit.expr(expr.lhs);
            visit.expr(expr.rhs);
        }
        ExprTerm::Err => {}
    }
}

/// Walks an expression in postorder.
///
/// This is useful for visitors that construct a parent value from already-visited
/// child values, such as IR lowering. Parentheses are treated as transparent syntax.
pub fn walk_expr_postorder<'src>(visit: &mut impl Visitor<'src>, id: ExprId) {
    match visit.ast().exprs().at(id).term() {
        ExprTerm::Paren(expr) => walk_expr_postorder(visit, expr.inner),
        ExprTerm::SelfRef(expr) => visit.expr_self_ref(id, expr),
        ExprTerm::Traversal(expr) => visit.expr_traversal(id, expr),
        ExprTerm::TypeRef(expr) => visit.expr_type_ref(id, expr),
        ExprTerm::UsersetTypeRef(expr) => visit.expr_userset_type_ref(id, expr),
        ExprTerm::Binary(expr) => {
            walk_expr_postorder(visit, expr.lhs);
            walk_expr_postorder(visit, expr.rhs);
            visit.expr_binary(id, expr);
        }
        ExprTerm::Err => {}
    }
}
