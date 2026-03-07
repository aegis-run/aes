use aes_ast_macros::ast_node;
use aes_foundation::Span;

use crate::ExprId;

#[ast_node]
pub struct TypeDef {
    span: Span,
    name: Span,
    lets: LetMemberRange,
    defs: DefMemberRange,
}

#[ast_node]
pub struct LetMember {
    span: Span,
    name: Span,
    expr: ExprId,
}

#[ast_node]
pub struct DefMember {
    span: Span,
    name: Span,
    expr: ExprId,
}
