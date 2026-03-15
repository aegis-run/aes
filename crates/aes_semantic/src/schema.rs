use aes_foundation::symbols::SymbolMap;
use rustc_hash::FxHashSet;

use crate::{
    PermissionInterner, PermissionSymbol, RelationInterner, RelationSymbol, TypeInterner,
    TypeMarker, TypeSymbol,
};

#[derive(Debug, Clone)]
pub struct Schema<'src> {
    pub(crate) types: SymbolMap<'src, TypeMarker, TypeSchema>,
    pub(crate) types_int: TypeInterner<'src>,
    pub(crate) relations_int: RelationInterner<'src>,
    pub(crate) permissions_int: PermissionInterner<'src>,
}

impl<'src> Schema<'src> {
    pub fn has_type(&self, name: TypeSymbol) -> bool {
        self.types.get(name).is_some()
    }

    pub fn has_relation(&self, ty: TypeSymbol, relation: RelationSymbol) -> bool {
        self.types
            .get(ty)
            .map(|it| it.relations.contains(&relation))
            .unwrap_or(false)
    }

    pub fn has_permission(&self, ty: TypeSymbol, permission: PermissionSymbol) -> bool {
        self.types
            .get(ty)
            .map(|it| it.permissions.contains(&permission))
            .unwrap_or(false)
    }

    pub fn has_member(&self, ty: TypeSymbol, name: &str) -> bool {
        let has_rel = self
            .relations_int
            .get(name)
            .is_some_and(|sym| self.has_relation(ty, sym));

        let has_perm = self
            .permissions_int
            .get(name)
            .is_some_and(|sym| self.has_permission(ty, sym));

        has_rel || has_perm
    }

    pub fn types(&self) -> impl Iterator<Item = TypeSymbol> + '_ {
        self.types.keys()
    }

    pub fn relations_of(&self, ty: TypeSymbol) -> impl Iterator<Item = RelationSymbol> + '_ {
        self.types
            .get(ty)
            .into_iter()
            .flat_map(|t| t.relations.iter().copied())
    }

    pub fn permissions_of(&self, ty: TypeSymbol) -> impl Iterator<Item = PermissionSymbol> + '_ {
        self.types
            .get(ty)
            .into_iter()
            .flat_map(|t| t.permissions.iter().copied())
    }

    // Resolution helpers
    pub fn resolve_type_name(&self, sym: TypeSymbol) -> &'src str {
        self.types_int.resolve(sym)
    }

    pub fn resolve_relation_name(&self, sym: RelationSymbol) -> &'src str {
        self.relations_int.resolve(sym)
    }

    pub fn resolve_permission_name(&self, sym: PermissionSymbol) -> &'src str {
        self.permissions_int.resolve(sym)
    }

    pub fn types_interner(&self) -> &TypeInterner<'src> {
        &self.types_int
    }

    pub fn relations_interner(&self) -> &RelationInterner<'src> {
        &self.relations_int
    }

    pub fn permissions_interner(&self) -> &PermissionInterner<'src> {
        &self.permissions_int
    }
}

#[derive(Debug, Clone)]
pub struct TypeSchema {
    pub(crate) relations: FxHashSet<RelationSymbol>,
    pub(crate) permissions: FxHashSet<PermissionSymbol>,
}
