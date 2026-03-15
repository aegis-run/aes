use aes_ast_macros::ast_node;
use aes_foundation::Span;

/// An expression in a `let` or `def` body.
///
/// # Syntax
/// ```aes
/// let x = user | group::member;
/// def y = .owner | .folder.viewer;
/// ```
#[ast_node]
pub struct Expr {
    span: Span,
    term: ExprTerm,
}

/// Binary operator for combining expressions.
///
/// Precedence (high to low): `-` > `&` > `|`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Union,        // `|`
    Intersection, // `&`
    Exclusion,    // `-`
}

/// Parenthesized expression: `(expr)`
#[derive(Debug, Clone, Copy)]
pub struct ExprTermParen {
    pub inner: ExprId,
}

/// Self-reference: `.relation`
///
/// Valid only in `def` bodies.
#[derive(Debug, Clone, Copy)]
pub struct ExprTermSelfRef {
    pub span: Span,
}

/// Traversal: `.relation.permission`
///
/// Valid only in `def` bodies.
#[derive(Debug, Clone, Copy)]
pub struct ExprTermTraversal {
    pub relation: Span,
    pub permission: Span,
}

/// Type reference: `user`
///
/// Valid only in `let` bodies (specifies subject type).
#[derive(Debug, Clone, Copy)]
pub struct ExprTermTypeRef {
    pub span: Span,
}

/// Userset type reference: `group::member`
///
/// Valid only in `let` bodies (references relation on another type).
#[derive(Debug, Clone, Copy)]
pub struct ExprTermUsersetTypeRef {
    pub ty: Span,
    pub member: Span,
}

/// Binary expression: `left op right`
#[derive(Debug, Clone, Copy)]
pub struct ExprTermBinary {
    pub op: BinaryOp,
    pub lhs: ExprId,
    pub rhs: ExprId,
}

/// The term (operation) of an [`Expr`].
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

    /// `.relation`
    pub const fn self_ref(span: Span) -> Self {
        ExprTerm::SelfRef(ExprTermSelfRef { span })
    }

    pub const fn traversal(relation: Span, permission: Span) -> Self {
        ExprTerm::Traversal(ExprTermTraversal {
            relation,
            permission,
        })
    }

    /// `type::relation`
    pub const fn userset_type_ref(ty: Span, member: Span) -> Self {
        ExprTerm::UsersetTypeRef(ExprTermUsersetTypeRef { ty, member })
    }

    pub const fn paren(inner: ExprId) -> Self {
        ExprTerm::Paren(ExprTermParen { inner })
    }

    /// `left op right`
    pub const fn binary(op: BinaryOp, lhs: ExprId, rhs: ExprId) -> Self {
        ExprTerm::Binary(ExprTermBinary { op, lhs, rhs })
    }
}
