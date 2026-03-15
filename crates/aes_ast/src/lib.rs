//! AST for the Aegis schema language.
//!
//! # Architecture
//! The Aegis AST uses an **Arena / Structure-of-Arrays (SoA)** design. Rather than traditional
//! tree structures with `Box<Node>` pointers, the tree is flattened into contiguous memory pools
//! within the root [`Ast`] object. Nodes reference their children via strongly-typed `u32` indices
//! (like [`ExprId`] or [`TypeDefId`]). This provides excellent cache locality and avoids complex
//! Rust borrow-checker lifetimes.
//!
//! # Construction & Traversal
//! * **Building:** To construct an `Ast`, use the [`AstBuilder`]. It provides a controlled interface
//!   to append nodes into their respective pools and retrieve their IDs.
//! * **Visiting:** To walk the AST, implement the [`visit::Visitor`] trait and pass it to
//!   one of the utility functions like [`visit::schema`]. The visitor pattern will automatically
//!   traverse relationships (e.g., following `ExprId` children within a binary expression)
//!   and dispatch callbacks to your visitor.
//!
//! # Structure Snapshot
//!
//! ```aes
//! type user {}
//!
//! type team {
//!     let member = user;
//!     def admin = .member;
//! }
//!
//! test "access_check" {
//!     relations {
//!         team("t1").member: user("alice");
//!     }
//!     assert(team("t1").admin(user("alice")));
//! }
//! ```

mod ast;
mod expr;
mod test_def;
mod type_def;

pub use ast::{Ast, AstBuilder, Instance};

pub use expr::{
    BinaryOp, Expr, ExprTerm, ExprTermBinary, ExprTermParen, ExprTermSelfRef, ExprTermTraversal,
    ExprTermTypeRef, ExprTermUsersetTypeRef,
};

pub use type_def::{DefMember, LetMember, TypeDef};

pub use test_def::{Assert, AssertionKind, Relation, Subject, TestDef};

pub use type_def::{
    DefMemberId, DefMemberPool, DefMemberPoolBuilder, DefMemberRange, DefMemberRef, LetMemberId,
    LetMemberPool, LetMemberPoolBuilder, LetMemberRange, LetMemberRef, TypeDefId, TypeDefPool,
    TypeDefPoolBuilder, TypeDefRange, TypeDefRef,
};

pub use expr::{ExprId, ExprPool, ExprPoolBuilder, ExprRange, ExprRef};

pub use test_def::{
    AssertId, AssertPool, AssertPoolBuilder, AssertRange, AssertRef, RelationId, RelationPool,
    RelationPoolBuilder, RelationRange, RelationRef, SubjectId, SubjectPool, SubjectPoolBuilder,
    SubjectRange, SubjectRef, TestDefId, TestDefPool, TestDefPoolBuilder, TestDefRange, TestDefRef,
};
