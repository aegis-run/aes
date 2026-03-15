use aes_ast_macros::ast_node;
use aes_foundation::Span;

use crate::Instance;

/// A `test` definition.
///
/// # Syntax
/// ```aes
/// test "access_check" {
///     relations {
///         user("alice").friend: user("bob");
///     }
///     assert(user("alice").friend(user("bob")));
///     assert_not(user("bob").friend(user("alice")));
/// }
/// ```
#[ast_node]
pub struct TestDef {
    span: Span,
    name: Span,
    relations: RelationRange,
    asserts: AssertRange,
}

/// A relation statement in a test's `relations` block.
///
/// # Syntax
/// ```aes
/// user("alice").friend: user("bob");
/// team("t1").member: user("alice")::member;
/// ```
#[ast_node]
pub struct Relation {
    span: Span,
    resource: Instance,
    relation: Span,
    subject: SubjectId,
}

/// Assertion kind: `assert` or `assert_not`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertionKind {
    Assert,
    AssertNot,
}

/// An assertion in a test.
///
/// # Syntax
/// ```aes
/// assert(object.permission(subject));
/// assert_not(object.permission(subject));
/// ```
#[ast_node]
pub struct Assert {
    span: Span,
    kind: AssertionKind,
    resource: Instance,
    permission: Span,
    actor: Instance,
}

/// A subject in a relation statement.
///
/// # Syntax
/// ```aes
/// user("alice")          // direct subject
/// team("t1")::member     // userset reference
/// ```
#[ast_node]
pub struct Subject {
    span: Span,
    instance: Instance,
    permission: Option<Span>,
}
