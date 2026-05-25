use crate::v1;
use prost::Message;

impl v1::Schema {
    /// Creates a new schema from a list of type definitions.
    pub fn new(types: Vec<v1::TypeDefinition>) -> Self {
        Self { types }
    }

    pub fn encode_bytes(&self) -> Vec<u8> {
        self.encode_to_vec()
    }
}

impl v1::TypeDefinition {
    /// Creates a new type definition with pre-allocated vectors for relations and permissions.
    pub fn with_capacity(
        name: impl Into<String>,
        relation_capacity: usize,
        permission_capacity: usize,
    ) -> Self {
        Self {
            name: name.into(),
            relations: Vec::with_capacity(relation_capacity),
            permissions: Vec::with_capacity(permission_capacity),
        }
    }
}

impl v1::Relation {
    /// Creates a new relation with the specified name and actors.
    pub fn new(name: impl Into<String>, actors: Vec<v1::ActorType>) -> Self {
        Self {
            name: name.into(),
            actors,
        }
    }
}

impl v1::Permission {
    /// Creates a new permission with the specified name and expression.
    pub fn new(name: impl Into<String>, expr: Option<v1::Expression>) -> Self {
        Self {
            name: name.into(),
            expr,
        }
    }
}

impl v1::ActorType {
    /// Creates a direct actor reference (e.g., `user`).
    pub fn direct(name: impl Into<String>) -> Self {
        Self {
            actor: Some(v1::actor_type::Actor::Direct(name.into())),
        }
    }

    /// Creates a userset actor reference (e.g., `group::member`).
    pub fn userset(ty: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            actor: Some(v1::actor_type::Actor::Userset(v1::UsersetType {
                r#type: ty.into(),
                member: member.into(),
            })),
        }
    }
}

impl v1::Expression {
    /// Creates a self-reference expression (e.g., `.owner`).
    pub fn self_ref(relation: impl Into<String>) -> Self {
        Self {
            kind: Some(v1::expression::Kind::Term(v1::TermExpr {
                term: Some(v1::term_expr::Term::SelfRef(v1::SelfRef {
                    relation: relation.into(),
                })),
            })),
        }
    }

    /// Creates a traversal expression (e.g., `.parent.view`).
    pub fn traversal(relation: impl Into<String>, permission: impl Into<String>) -> Self {
        Self {
            kind: Some(v1::expression::Kind::Term(v1::TermExpr {
                term: Some(v1::term_expr::Term::Traversal(v1::Traversal {
                    relation: relation.into(),
                    permission: permission.into(),
                })),
            })),
        }
    }

    /// Creates a union expression from a list of terms.
    pub fn union(terms: Vec<Self>) -> Self {
        Self {
            kind: Some(v1::expression::Kind::Union(v1::UnionExpr { terms })),
        }
    }

    /// Creates an intersection expression from a list of terms.
    pub fn intersection(terms: Vec<Self>) -> Self {
        Self {
            kind: Some(v1::expression::Kind::Intersection(v1::IntersectionExpr {
                terms,
            })),
        }
    }

    /// Creates a difference expression (exclusion).
    pub fn difference(lhs: Self, rhs: Self) -> Self {
        Self {
            kind: Some(v1::expression::Kind::Difference(Box::new(
                v1::DifferenceExpr {
                    lhs: Some(Box::new(lhs)),
                    rhs: Some(Box::new(rhs)),
                },
            ))),
        }
    }
}
