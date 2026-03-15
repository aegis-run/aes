//! Core primitives and foundational data structures for the Aegis compiler.
//!
//! `aes_foundation` acts as the shared backbone across parsing, semantic analysis,
//! and further compilation stages. It provides the following key utilities:
//!
//! * **Zero-Allocation Graph IDs**: [`Id`], [`Range`], and [`Checkpoint`] for Arena/SoA structures.
//! * **Symbol Interning**: [`interner::Interner`] and [`symbols::SymbolId`] for zero-cost string comparisons.
//! * **Diagnostics**: A rich [`Diagnostic`] builder wrapping `miette`, and a [`Reporter`] interface for emitting them.
//! * **Source Span Tracking**: The highly-optimzed [`Span`] struct to track byte ranges.
mod const_assert;
mod diagnostic;
mod id;
pub mod interner;
pub mod prelude;
mod span;
pub mod symbols;

pub use diagnostic::*;
pub use id::*;
pub use span::Span;
