use aes_allocator::{Allocator, Vec};
use aes_foundation::Span;
use rustc_hash::FxHashMap;

use crate::schema::{Schema, TypeSchema};
use crate::{
    PermissionInterner, PermissionSymbol, RelationInterner, RelationSymbol, TypeInterner,
    TypeMarker, TypeSymbol,
};

use aes_foundation::symbols::SymbolMap;

pub(crate) struct SemanticIndex<'src> {
    alloc: &'src Allocator,

    symbols: SymbolMap<'src, TypeMarker, (aes_ast::TypeDefId, Span)>,
    members: TypeIndex<'src>,

    types: TypeInterner<'src>,
    relations: RelationInterner<'src>,
    permissions: PermissionInterner<'src>,
}

pub type Collision = Span;

impl<'src> SemanticIndex<'src> {
    pub fn with_capacity(alloc: &'src Allocator, capacity: usize) -> Self {
        Self {
            alloc,
            symbols: SymbolMap::with_capacity(alloc, capacity),
            members: TypeIndex::with_capacity(alloc, capacity),

            types: TypeInterner::with_capacity(capacity),
            relations: RelationInterner::with_capacity(2 * capacity),
            permissions: PermissionInterner::with_capacity(2 * capacity),
        }
    }

    pub fn declare_type(
        &mut self,
        id: aes_ast::TypeDefId,
        span: Span,
        name: &'src str,
    ) -> Option<Collision> {
        let sym = self.types.intern(name);

        if let Some(&(.., prev_span)) = self.symbols.get(sym) {
            return Some(prev_span);
        };

        self.symbols.push_sequential(sym, (id, span));
        self.members.push(span, sym);

        None
    }

    pub fn declare_relation(
        &mut self,
        scope: aes_ast::TypeDefId,
        span: Span,
        name: &'src str,
        expr_id: aes_ast::ExprId,
    ) -> Option<Collision> {
        let sym = self.relations.intern(name);
        let relations = self.members.relations.get_mut(scope.as_index())?;
        let prev = relations.get(&sym).map(|&(prev, _)| prev);
        relations.insert(sym, (span, expr_id));
        prev
    }

    pub fn declare_permission(
        &mut self,
        scope: aes_ast::TypeDefId,
        span: Span,
        name: &'src str,
        expr_id: aes_ast::ExprId,
    ) -> Option<Collision> {
        let sym = self.permissions.intern(name);
        let permissions = self.members.permissions.get_mut(scope.as_index())?;
        let prev = permissions.get(&sym).map(|&(prev, _)| prev);
        permissions.insert(sym, (span, expr_id));
        prev
    }

    pub fn relation_collision(
        &self,
        scope: aes_ast::TypeDefId,
        name: &'src str,
    ) -> Option<Collision> {
        let sym = self.relations.get(name)?;
        self.members
            .relations
            .get(scope.as_index())?
            .get(&sym)
            .map(|&(s, _)| s)
    }

    pub fn permission_collision(
        &self,
        scope: aes_ast::TypeDefId,
        name: &'src str,
    ) -> Option<Collision> {
        let sym = self.permissions.get(name)?;
        self.members
            .permissions
            .get(scope.as_index())?
            .get(&sym)
            .map(|&(s, _)| s)
    }

    pub fn type_(&self, name: &str) -> Option<aes_ast::TypeDefId> {
        let sym = self.types.get(name)?;
        let (id, _) = self.symbols.get(sym)?;
        Some(*id)
    }

    pub fn type_name(&self, id: aes_ast::TypeDefId) -> Option<&str> {
        self.members
            .names
            .get(id.as_index())
            .map(|&sym| self.types.resolve(sym))
    }

    pub fn relation(
        &self,
        scope: aes_ast::TypeDefId,
        name: &str,
    ) -> Option<(Span, aes_ast::ExprId)> {
        let sym = self.relations.get(name)?;
        self.relation_by_symbol(scope, sym)
    }

    pub fn has_relation(&self, scope: aes_ast::TypeDefId, name: &str) -> bool {
        self.relation(scope, name).is_some()
    }

    fn relation_by_symbol(
        &self,
        scope: aes_ast::TypeDefId,
        name: RelationSymbol,
    ) -> Option<(Span, aes_ast::ExprId)> {
        self.members
            .relations
            .get(scope.as_index())?
            .get(&name)
            .copied()
    }

    pub fn permission(
        &self,
        scope: aes_ast::TypeDefId,
        name: &str,
    ) -> Option<(Span, aes_ast::ExprId)> {
        let sym = self.permissions.get(name)?;
        self.permission_by_symbol(scope, sym)
    }

    pub fn has_permission(&self, scope: aes_ast::TypeDefId, name: &str) -> bool {
        self.permission(scope, name).is_some()
    }

    fn permission_by_symbol(
        &self,
        scope: aes_ast::TypeDefId,
        name: PermissionSymbol,
    ) -> Option<(Span, aes_ast::ExprId)> {
        self.members
            .permissions
            .get(scope.as_index())?
            .get(&name)
            .copied()
    }

    pub fn has_member(&self, scope: aes_ast::TypeDefId, name: &str) -> bool {
        self.relation(scope, name).is_some() || self.permission(scope, name).is_some()
    }

    pub fn into_schema(self) -> Schema<'src> {
        let mut types = SymbolMap::new(self.alloc);

        for i in 0..self.members.names.len() {
            let relations = self.members.relations[i].keys().copied().collect();
            let permissions = self.members.permissions[i].keys().copied().collect();

            types.push_sequential(
                TypeSymbol::new(i as u32),
                TypeSchema {
                    relations,
                    permissions,
                },
            );
        }

        Schema {
            types,
            types_int: self.types,
            relations_int: self.relations,
            permissions_int: self.permissions,
        }
    }
}

struct TypeIndex<'alloc> {
    spans: Vec<'alloc, Span>,
    names: Vec<'alloc, TypeSymbol>,
    relations: Vec<'alloc, FxHashMap<RelationSymbol, (Span, aes_ast::ExprId)>>,
    permissions: Vec<'alloc, FxHashMap<PermissionSymbol, (Span, aes_ast::ExprId)>>,
}

impl<'alloc> TypeIndex<'alloc> {
    fn with_capacity(alloc: &'alloc Allocator, capacity: usize) -> Self {
        Self {
            spans: Vec::with_capacity_in(capacity, alloc),
            names: Vec::with_capacity_in(capacity, alloc),
            relations: Vec::with_capacity_in(capacity, alloc),
            permissions: Vec::with_capacity_in(capacity, alloc),
        }
    }

    fn push(&mut self, span: Span, name: TypeSymbol) {
        self.spans.push(span);
        self.names.push(name);
        self.relations.push(FxHashMap::default());
        self.permissions.push(FxHashMap::default());
    }
}

#[cfg(test)]
mod tests {
    use aes_ast::{ExprId, TypeDefId};
    use aes_testing::SPAN;

    use super::*;

    fn type_id(i: u32) -> TypeDefId {
        TypeDefId::new(i)
    }
    fn expr_id(i: u32) -> ExprId {
        ExprId::new(i)
    }

    mod declare_type {
        use super::*;

        #[test]
        fn first_time_returns_none() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            assert!(idx.declare_type(type_id(0), SPAN, "user").is_none());
        }

        #[test]
        fn duplicate_returns_previous_span() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            let first_span = Span::empty(10);
            idx.declare_type(type_id(0), first_span, "user");
            let collision = idx.declare_type(type_id(1), Span::empty(20), "user");

            assert_eq!(collision.unwrap().start(), first_span.start());
        }

        #[test]
        fn duplicate_does_not_overwrite() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            idx.declare_type(type_id(1), Span::empty(10), "user"); // collision

            assert_eq!(idx.type_("user"), Some(type_id(0)));
        }

        #[test]
        fn distinct_all_resolve() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            idx.declare_type(type_id(1), Span::empty(10), "group");

            assert_eq!(idx.type_("user"), Some(type_id(0)));
            assert_eq!(idx.type_("group"), Some(type_id(1)));
        }

        #[test]
        fn type_unknown_name_returns_none() {
            let alloc = Allocator::new();
            let idx = SemanticIndex::with_capacity(&alloc, 8);

            assert!(idx.type_("unknown").is_none());
        }
    }

    mod declare_relation {
        use super::*;

        #[test]
        fn first_time_returns_none() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");

            assert!(
                idx.declare_relation(type_id(0), Span::empty(5), "member", expr_id(0))
                    .is_none()
            );
        }

        #[test]
        fn duplicate_returns_previous_span() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            let first_span = Span::empty(5);
            idx.declare_relation(type_id(0), first_span, "member", expr_id(0));
            let collision = idx.declare_relation(type_id(0), Span::empty(15), "member", expr_id(1));

            assert_eq!(collision.unwrap().start(), first_span.start());
        }

        #[test]
        fn distinct_all_resolve() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");

            assert!(
                idx.declare_relation(type_id(0), Span::empty(0), "member", expr_id(0))
                    .is_none()
            );
            assert!(
                idx.declare_relation(type_id(0), Span::empty(5), "owner", expr_id(1))
                    .is_none()
            );
        }

        #[test]
        fn scoped_to_type() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            idx.declare_type(type_id(1), Span::empty(5), "group");

            assert!(
                idx.declare_relation(type_id(0), Span::empty(10), "member", expr_id(0))
                    .is_none()
            );
            assert!(
                idx.declare_relation(type_id(1), Span::empty(20), "member", expr_id(1))
                    .is_none()
            );
        }

        #[test]
        fn unknown_scope_returns_none() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            assert!(
                idx.declare_relation(type_id(99), Span::empty(0), "member", expr_id(0))
                    .is_none()
            );
        }
    }

    mod declare_permission {
        use super::*;

        #[test]
        fn first_time_returns_none() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            assert!(
                idx.declare_permission(type_id(0), Span::empty(5), "read", expr_id(0))
                    .is_none()
            );
        }

        #[test]
        fn duplicate_returns_previous_span() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            let first_span = Span::empty(5);
            idx.declare_permission(type_id(0), first_span, "read", expr_id(0));
            let collision = idx.declare_permission(type_id(0), Span::empty(15), "read", expr_id(1));

            assert_eq!(collision.unwrap().start(), first_span.start());
        }

        #[test]
        fn scoped_to_type() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            idx.declare_type(type_id(1), Span::empty(5), "group");

            assert!(
                idx.declare_permission(type_id(0), Span::empty(10), "read", expr_id(0))
                    .is_none()
            );
            assert!(
                idx.declare_permission(type_id(1), Span::empty(20), "read", expr_id(1))
                    .is_none()
            );
        }

        #[test]
        fn relation_and_permission_namespaces_are_independent() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");

            assert!(
                idx.declare_relation(type_id(0), Span::empty(5), "read", expr_id(0))
                    .is_none()
            );
            assert!(
                idx.declare_permission(type_id(0), Span::empty(10), "read", expr_id(1))
                    .is_none()
            );
        }
    }

    mod into_schema {
        use super::*;

        #[test]
        fn preserves_relations_per_type() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            idx.declare_relation(type_id(0), Span::empty(10), "member", expr_id(0));
            idx.declare_type(type_id(1), Span::empty(5), "group");
            idx.declare_relation(type_id(1), Span::empty(15), "owner", expr_id(1));

            let schema = idx.into_schema();
            let user_sym = schema.types_int.get("user").unwrap();
            let group_sym = schema.types_int.get("group").unwrap();
            let member_sym = schema.relations_int.get("member").unwrap();
            let owner_sym = schema.relations_int.get("owner").unwrap();

            let user_schema = schema.types.get(user_sym).unwrap();
            let group_schema = schema.types.get(group_sym).unwrap();

            assert!(user_schema.relations.contains(&member_sym));
            assert!(!user_schema.relations.contains(&owner_sym));
            assert!(group_schema.relations.contains(&owner_sym));
            assert!(!group_schema.relations.contains(&member_sym));
        }

        #[test]
        fn preserves_permissions_per_type() {
            let alloc = Allocator::new();
            let mut idx = SemanticIndex::with_capacity(&alloc, 8);

            idx.declare_type(type_id(0), Span::empty(0), "user");
            idx.declare_type(type_id(1), Span::empty(5), "group");
            idx.declare_permission(type_id(0), Span::empty(10), "read", expr_id(0));
            idx.declare_permission(type_id(1), Span::empty(15), "write", expr_id(1));

            let schema = idx.into_schema();
            let user_sym = schema.types_int.get("user").unwrap();
            let group_sym = schema.types_int.get("group").unwrap();
            let read_sym = schema.permissions_int.get("read").unwrap();
            let write_sym = schema.permissions_int.get("write").unwrap();

            let user_schema = schema.types.get(user_sym).unwrap();
            let group_schema = schema.types.get(group_sym).unwrap();

            assert!(user_schema.permissions.contains(&read_sym));
            assert!(!user_schema.permissions.contains(&write_sym));
            assert!(group_schema.permissions.contains(&write_sym));
            assert!(!group_schema.permissions.contains(&read_sym));
        }

        #[test]
        fn empty_index_produces_empty_schema() {
            let alloc = Allocator::new();
            let idx = SemanticIndex::with_capacity(&alloc, 8);

            let schema = idx.into_schema();
            assert!(schema.types_int.is_empty());
            assert!(schema.relations_int.is_empty());
            assert!(schema.permissions_int.is_empty());
        }
    }
}
