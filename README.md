# Aegis: The `.aes` Schema Toolchain

_A reference compiler and formal verification suite for relation-based access control._

---

This repository serves as the archival record for the `aes` toolchain, the primary
compiler frontend for the Aegis authorization system. It provides the formal
specification, semantic analysis, and intermediate representation (IR) export pipeline
required to transform declarative domain models into executable authorization graphs.

## The Repository Structure

The workspace is organized into discrete, functionally isolated crates to support
both standalone CLI operations and embedded compiler integration:

- `aes_parser/` Lexical analysis and LL(1) recursive-descent parsing.
- `aes_semantic/` Symbol resolution and formal semantic verification.
- `aes_compiler/` Orchestration of the compilation pipeline and IR export.
- `aes_ir/` Protobuf-backed intermediate representation for the Aegis v1 schema.
- `aes_foundation/` Core primitives for span management and diagnostics.
- `aes_testing/` Facilities for property-based testing and snapshot validation.

---

## Technical Specification

The `.aes` language formalizes Relationship-Based Access Control (ReBAC) through
type definitions, relations (`let`), and algebraic permissions (`def`).

### Formal Grammar (EBNF)

```ebnf
(* TOP LEVEL *)
schema = type_def* ;

(* SCHEMA DEFINITIONS *)
type_def = "type", ident, "{", member*, "}" ;
member   = ("let" | "def"), ident, "=", expr, ";" ;

expr = term, (("|" | "&" | "-"), term)* ;
term = ".", ident, (".", ident)?
     | ident, ("::", ident)?
     | "(", expr, ")" ;

(* COMMON PRIMITIVES *)
ident  = letter, (letter | digit | "_")* ;
```

### Model Example

```aes
type user {}

type group {
  let member = user;
}

type organization {
  let admin = user;
  let member = user | group::member;
}

type directory {
  let organization = organization;
  let viewer = user | group::member;
  let editor = user;
  let parent = directory;

  def view = .viewer | .editor | .parent.view;
  def edit = .editor & .organization.member;
  def delete = .organization.admin - .viewer;
}

type document {
  let directory = directory;
  let owner = user;
  let commenter = user | group::member;

  def view = .owner | .commenter | .directory.view;
  def edit = .owner | .directory.edit;
  def comment = .commenter | .owner;
  def delete = .owner & .directory.delete;
}
```

---

## Reproducibility and Procedures

The following procedures ensure the structural integrity and deterministic output
of the toolchain.

### Environmental Integrity

The project utilizes Nix to ensure a deterministic Rust toolchain and development
environment.

```bash
nix develop
```

### Toolchain Operations

The `aes` executable provides the primary interface for schema management and
verification.

_Operation A: IR Export_

```bash
aes export ./schema.aes --server http://localhost:43615
```

_Operation B: Diagnostic Dump_

```bash
aes dump ./schema.aes
```

### Integrity Verification

The project mandates strict quality control via the `just` command runner.

_Verification A: Static Analysis (Clippy)_

```bash
just lint
```

_Verification B: Functional Testing (Nextest)_

```bash
just test
```

_Verification C: Snapshot Integrity (Insta)_

```bash
just snap-test
```

---

## Technical Reference

For the full implementation and empirical analysis, refer to the sibling repository
and the archival thesis:  
[`Aegis: A Centralized Authorization System Based on Google Zanzibar.`](https://github.com/aegis-run/thesis)

_Faculty of Mathematics and Computer Science, University of Bucharest._
