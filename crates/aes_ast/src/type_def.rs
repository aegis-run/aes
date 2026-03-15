use aes_ast_macros::ast_node;
use aes_foundation::Span;

use crate::ExprId;

/// A `type` definition.
///
/// # Syntax
/// ```aes
/// type user {
///     let friend = user;
///     def mutual_friend = .friend::friend;
/// }
/// ```
#[ast_node]
pub struct TypeDef {
    span: Span,
    name: Span,
    lets: LetMemberRange,
    defs: DefMemberRange,
}

/// A `let` member (stored relation).
///
/// # Syntax
/// ```aes
/// let member = user | team::member;
/// ```
#[ast_node]
pub struct LetMember {
    span: Span,
    name: Span,
    expr: ExprId,
}

/// A `def` member (computed permission).
///
/// # Syntax
/// ```aes
/// def admin = .owner | .parent.admin;
/// ```
#[ast_node]
pub struct DefMember {
    span: Span,
    name: Span,
    expr: ExprId,
}
