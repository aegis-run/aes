use aes_foundation::{Diagnostic, Span};

pub fn duplicate_type(span: Span, prev: Span, name: &str) -> Diagnostic {
    Diagnostic::error(format!("duplicate type `{name}`"))
        .with_code("aes::semantic", "duplicate_type")
        .with_label(span.label("duplicate defined here"))
        .and_label(prev.label("previously defined here"))
        .with_help(format!(
            "rename one of the `{name}` types to make them unique"
        ))
}

pub fn duplicate_relation(span: Span, prev: Span, name: &str) -> Diagnostic {
    Diagnostic::error(format!("duplicate relation `{name}`"))
        .with_code("aes::semantic", "duplicate_relation")
        .with_label(span.label("duplicate defined here"))
        .and_label(prev.label("previously defined here"))
        .with_help(format!(
            "rename one of the `{name}` relations or merge them into one"
        ))
}

pub fn duplicate_permission(span: Span, prev: Span, name: &str) -> Diagnostic {
    Diagnostic::error(format!("duplicate permission `{name}`"))
        .with_code("aes::semantic", "duplicate_permission")
        .with_label(span.label("duplicate defined here"))
        .and_label(prev.label("previously defined here"))
        .with_help(format!(
            "rename one of the `{name}` permissions or merge them into one"
        ))
}

pub fn relation_permission_name_collision(
    perm_span: Span,
    rel_span: Span,
    name: &str,
) -> Diagnostic {
    Diagnostic::error(format!(
        "`{name}` is defined as both a relation and a permission"
    ))
    .with_code("aes::semantic", "relation_permission_name_collision")
    .with_label(perm_span.label("defined as a permission here"))
    .and_label(rel_span.label("previously defined as a relation here"))
    .with_help(format!(
        "`let` defines stored relations, `def` defines computed permissions \
             — they cannot share the name `{name}`"
    ))
}

pub fn unknown_type(span: Span) -> Diagnostic {
    Diagnostic::error("unknown type")
        .with_code("aes::semantic", "unknown_type")
        .with_label(span.label("this type is not defined in the schema"))
}

pub fn unknown_member(span: Span, type_name: &str) -> Diagnostic {
    Diagnostic::error(format!("unknown member on type `{type_name}`"))
        .with_code("aes::semantic", "unknown_member")
        .with_label(span.label(format!("not defined on `{type_name}`")))
}

pub fn unknown_relation(span: Span) -> Diagnostic {
    Diagnostic::error("unknown relation")
        .with_code("aes::semantic", "unknown_relation")
        .with_label(span.label("not defined on this type"))
        .with_help("add a `let` member with this name to the enclosing type")
}

pub fn unknown_permission_on_type(
    span: Span,
    permission_name: &str,
    type_name: &str,
) -> Diagnostic {
    Diagnostic::error(format!(
        "unknown permission or relation `{permission_name}` on type `{type_name}`"
    ))
    .with_code("aes::semantic", "unknown_permission_on_type")
    .with_label(span.label(format!("not defined on `{type_name}`")))
    .with_help(format!(
        "add `{permission_name}` as a `let` or `def` member to `{type_name}`"
    ))
}

pub fn self_ref_in_relation(span: Span) -> Diagnostic {
    Diagnostic::error("self-reference not allowed in `let` body")
        .with_code("aes::semantic", "self_ref_in_relation")
        .with_label(span.label("`.relation` syntax is not valid here"))
        .with_help(
            "`let` defines what external types can be stored as subjects — \
             use `def` for permissions that reference the type's own relations",
        )
}

pub fn type_ref_in_permission(span: Span) -> Diagnostic {
    Diagnostic::error("type reference not allowed in `def` body")
        .with_code("aes::semantic", "type_ref_in_permission")
        .with_label(span.label("bare type names are not valid here"))
        .with_help(
            "`def` defines computed permissions using `.relation` or `.relation.permission` — \
             use `let` to declare which types can be assigned to a relation",
        )
}

pub fn userset_ref_to_permission(span: Span, type_name: &str) -> Diagnostic {
    Diagnostic::error("userset references a computed permission, not a relation")
        .with_code("aes::semantic", "userset_ref_to_permission")
        .with_label(span.label(format!(
            "this is a `def` on `{type_name}`, but only `let` relations \
             can be used in userset references"
        )))
        .with_help(
            "userset syntax `type::member` requires `member` to be a `let` relation, \
             not a `def` permission",
        )
}

pub fn traversal_on_permission(span: Span) -> Diagnostic {
    Diagnostic::error("cannot traverse through a computed permission")
        .with_code("aes::semantic", "traversal_on_permission")
        .with_label(span.label("this is a `def`, but traversals require a `let` relation"))
        .with_help(
            "traversal syntax `.relation.permission` requires the first part to be \
             a `let` relation — computed permissions (`def`) cannot be traversed",
        )
}
