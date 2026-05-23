use proptest::prelude::*;

pub fn ident() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_]{0,7}")
        .unwrap()
        .prop_filter("not a keyword", |s| {
            !matches!(
                s.as_str(),
                "type" | "let" | "def" | "test" | "relations" | "assert" | "assert_not"
            )
        })
}

pub fn expr() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        ident().prop_map(|s| s),
        (ident(), ident()).prop_map(|(a, b)| format!("{a}::{b}")),
        ident().prop_map(|s| format!(".{s}")),
        (ident(), ident()).prop_map(|(a, b)| format!(".{a}.{b}")),
    ];

    leaf.prop_recursive(4, 16, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} | {b}")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} & {b}")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} - {b}")),
            inner.prop_map(|a| format!("({a})")),
        ]
    })
}

pub fn let_member() -> impl Strategy<Value = String> {
    (ident(), expr()).prop_map(|(name, expr)| format!("let {name} = {expr};"))
}

pub fn def_member() -> impl Strategy<Value = String> {
    (ident(), expr()).prop_map(|(name, expr)| format!("def {name} = {expr};"))
}

pub fn type_def() -> impl Strategy<Value = String> {
    (
        ident(),
        proptest::collection::vec(let_member(), 0..4),
        proptest::collection::vec(def_member(), 0..4),
    )
        .prop_map(|(name, lets, defs)| {
            let body = lets
                .into_iter()
                .chain(defs)
                .collect::<Vec<_>>()
                .join("\n    ");
            format!("type {name} {{\n    {body}\n}}")
        })
}

pub fn quoted_string() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_]+")
        .unwrap()
        .prop_map(|s| format!("\"{s}\""))
}

pub fn instance() -> impl Strategy<Value = String> {
    (ident(), quoted_string()).prop_map(|(ty, id)| format!("{ty}({id})"))
}

pub fn subject() -> impl Strategy<Value = String> {
    prop::strategy::Union::new(vec![
        instance().boxed(),
        (instance(), ident())
            .prop_map(|(inst, perm)| format!("{inst}::{perm}"))
            .boxed(),
    ])
}

pub fn relation_assign() -> impl Strategy<Value = String> {
    (ident(), subject()).prop_map(|(rel, subj)| format!(".{rel}:{subj};"))
}

pub fn relation_stmt() -> impl Strategy<Value = String> {
    prop_oneof![
        (instance(), ident(), subject()).prop_map(|(i, r, s)| format!("{i}.{r}:{s};")),
        (
            instance(),
            proptest::collection::vec(relation_assign(), 1..3)
        )
            .prop_map(|(inst, assigns)| { format!("{}.{{ {} }};", inst, assigns.join(" ")) })
    ]
}

pub fn assertion() -> impl Strategy<Value = String> {
    (prop::bool::ANY, instance(), ident(), instance()).prop_map(
        |(positive, resource, perm, actor)| {
            let kw = if positive { "assert" } else { "assert_not" };
            format!("{kw}( {resource}.{perm}( {actor} ) );")
        },
    )
}

pub fn test_def() -> impl Strategy<Value = String> {
    (
        quoted_string(),
        proptest::collection::vec(relation_stmt(), 0..4),
        proptest::collection::vec(assertion(), 0..4),
    )
        .prop_map(|(name, rels, asserts)| {
            let rels_body = rels.join("\n        ");
            let asserts_body = asserts.join("\n    ");
            format!(
                "test {name} {{\n    relations {{\n        {rels_body}\n    }}\n    {asserts_body}\n}}"
            )
        })
}

/// Generates a syntactically valid (but not necessarily semantically valid) program.
pub fn syntactic_program() -> impl Strategy<Value = String> {
    (
        proptest::collection::vec(type_def(), 0..4),
        proptest::collection::vec(test_def(), 0..2),
    )
        .prop_map(|(types, tests)| {
            types
                .into_iter()
                .chain(tests)
                .collect::<Vec<_>>()
                .join("\n\n")
        })
}
