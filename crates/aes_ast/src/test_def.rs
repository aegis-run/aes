use aes_ast_macros::ast_node;
use aes_foundation::Span;

#[ast_node]
pub struct TestDef {
    span: Span,
    name: Span,
    relations: RelationRange,
    asserts: AssertRange,
}

#[ast_node]
pub struct Relation {
    span: Span,
    resource: Instance,
    relation: Span,
    subject: SubjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertionKind {
    Assert,
    AssertNot,
}

#[ast_node]
pub struct Assert {
    span: Span,
    kind: AssertionKind,
    resource: Instance,
    permission: Span,
    actor: Instance,
}

#[ast_node]
pub struct Subject {
    span: Span,
    instance: Instance,
    permission: Option<Span>,
}
