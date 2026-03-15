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
pub struct ExprTermParen {
    pub inner: ExprId,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprTermSelfRef {
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprTermTraversal {
    pub relation: Span,
    pub permission: Span,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprTermTypeRef {
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprTermUsersetTypeRef {
    pub ty: Span,
    pub member: Span,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprTermBinary {
    pub op: BinaryOp,
    pub lhs: ExprId,
    pub rhs: ExprId,
}

#[derive(Debug, Clone, Copy)]
pub enum ExprTerm {
    Paren(ExprTermParen),
    SelfRef(ExprTermSelfRef),
    Traversal(ExprTermTraversal),
    TypeRef(ExprTermTypeRef),
    UsersetTypeRef(ExprTermUsersetTypeRef),
    Binary(ExprTermBinary),
    Err,
}

aes_foundation::const_assert!(std::mem::size_of::<ExprTerm>() == 20);

impl ExprTerm {

    pub const fn type_ref(span: Span) -> Self {
        ExprTerm::TypeRef(ExprTermTypeRef { span })
    }

    pub const fn self_ref(span: Span) -> Self {
        ExprTerm::SelfRef(ExprTermSelfRef { span })
    }


    pub const fn traversal(relation: Span, permission: Span) -> Self {
        ExprTerm::Traversal(ExprTermTraversal {
            relation,
            permission,
        })
    }

    pub const fn userset_type_ref(ty: Span, member: Span) -> Self {
        ExprTerm::UsersetTypeRef(ExprTermUsersetTypeRef { ty, member })
    }


    pub const fn paren(inner: ExprId) -> Self {
        ExprTerm::Paren(ExprTermParen { inner })
    }

    pub const fn binary(op: BinaryOp, lhs: ExprId, rhs: ExprId) -> Self {
        ExprTerm::Binary(ExprTermBinary { op, lhs, rhs })
    }
}
