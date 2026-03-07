use aes_ast_macros::ast_node;
use aes_foundation::Span;

#[ast_node]
pub struct Expr {
    span: Span,
    term: ExprTerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Union,
    Intersection,
    Exclusion,
}

#[derive(Debug, Clone, Copy)]
pub enum ExprTerm {
    Paren(ExprId),
    SelfRef(Span),
    Traversal {
        relation: Span,
        permission: Span,
    },
    TypeRef(Span),
    UsersetTypeRef {
        ty: Span,
        member: Span,
    },
    Binary {
        op: BinaryOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Err,
}
