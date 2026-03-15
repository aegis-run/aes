use aes_ast::{ExprId, ExprTerm};

use crate::Visitor;

pub fn walk_expr<'src>(visit: &mut impl Visitor<'src>, id: ExprId) {
    match visit.ast().exprs().at(id).term() {
        ExprTerm::Paren(expr) => visit.expr_paren(expr),
        ExprTerm::SelfRef(expr) => visit.expr_self_ref(expr),
        ExprTerm::Traversal(expr) => visit.expr_traversal(expr),
        ExprTerm::TypeRef(expr) => visit.expr_type_ref(expr),
        ExprTerm::UsersetTypeRef(expr) => visit.expr_userset_type_ref(expr),
        ExprTerm::Binary(expr) => {
            visit.expr_binary(expr);
            visit.expr(expr.lhs);
            visit.expr(expr.rhs);
        }
        ExprTerm::Err => {}
    }
}
