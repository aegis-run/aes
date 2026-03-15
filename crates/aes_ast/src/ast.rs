use aes_allocator::Allocator;
use aes_foundation::Span;

use crate::*;
#[derive(Debug, Clone, Copy)]
pub struct Ast<'src> {
    types: TypeDefPool<'src>,
    lets: LetMemberPool<'src>,
    defs: DefMemberPool<'src>,
    exprs: ExprPool<'src>,

    tests: TestDefPool<'src>,
    relations: RelationPool<'src>,
    subjects: SubjectPool<'src>,
    asserts: AssertPool<'src>,
}

impl<'src> Ast<'src> {
    pub fn types(&self) -> &TypeDefPool<'src> {
        &self.types
    }

    pub fn iter_types(&self) -> impl Iterator<Item = TypeDefRef<'_>> {
        self.types.range(TypeDefRange::new(
            TypeDefId::new(0),
            TypeDefId::new(self.types.len() as u32),
        ))
    }

    pub fn lets(&self) -> &LetMemberPool<'src> {
        &self.lets
    }

    pub fn defs(&self) -> &DefMemberPool<'src> {
        &self.defs
    }

    pub fn exprs(&self) -> &ExprPool<'src> {
        &self.exprs
    }

    pub fn tests(&self) -> &TestDefPool<'src> {
        &self.tests
    }

    pub fn iter_tests(&self) -> impl Iterator<Item = TestDefRef<'_>> {
        self.tests.range(TestDefRange::new(
            TestDefId::new(0),
            TestDefId::new(self.tests.len() as u32),
        ))
    }

    pub fn relations(&self) -> &RelationPool<'src> {
        &self.relations
    }

    pub fn subjects(&self) -> &SubjectPool<'src> {
        &self.subjects
    }

    pub fn asserts(&self) -> &AssertPool<'src> {
        &self.asserts
    }
}

#[derive(Debug)]
pub struct AstBuilder<'src> {
    pub types: TypeDefPoolBuilder<'src>,
    pub lets: LetMemberPoolBuilder<'src>,
    pub defs: DefMemberPoolBuilder<'src>,
    pub exprs: ExprPoolBuilder<'src>,

    pub tests: TestDefPoolBuilder<'src>,
    pub relations: RelationPoolBuilder<'src>,
    pub subjects: SubjectPoolBuilder<'src>,
    pub asserts: AssertPoolBuilder<'src>,
}

impl<'src> AstBuilder<'src> {
    pub fn new(alloc: &'src Allocator) -> Self {
        Self {
            types: TypeDefPoolBuilder::new(alloc),
            lets: LetMemberPoolBuilder::new(alloc),
            defs: DefMemberPoolBuilder::new(alloc),
            exprs: ExprPoolBuilder::new(alloc),

            tests: TestDefPoolBuilder::new(alloc),
            relations: RelationPoolBuilder::new(alloc),
            subjects: SubjectPoolBuilder::new(alloc),
            asserts: AssertPoolBuilder::new(alloc),
        }
    }

    pub fn type_def(
        &mut self,
        span: Span,
        name: Span,
        lets: LetMemberRange,
        defs: DefMemberRange,
    ) -> TypeDefId {
        self.types.push(span, name, lets, defs)
    }

    pub fn let_member(&mut self, span: Span, name: Span, expr: ExprId) -> LetMemberId {
        self.lets.push(span, name, expr)
    }

    pub fn def_member(&mut self, span: Span, name: Span, expr: ExprId) -> DefMemberId {
        self.defs.push(span, name, expr)
    }

    pub fn expr(&mut self, span: Span, term: ExprTerm) -> ExprId {
        self.exprs.push(span, term)
    }

    pub fn test_def(
        &mut self,
        span: Span,
        name: Span,
        relations: RelationRange,
        asserts: AssertRange,
    ) -> TestDefId {
        self.tests.push(span, name, relations, asserts)
    }

    pub fn relation(
        &mut self,
        span: Span,
        resource: Instance,
        relation: Span,
        subject: SubjectId,
    ) -> RelationId {
        self.relations.push(span, resource, relation, subject)
    }

    pub fn assert(
        &mut self,
        span: Span,
        kind: AssertionKind,
        resource: Instance,
        permission: Span,
        actor: Instance,
    ) -> AssertId {
        self.asserts.push(span, kind, resource, permission, actor)
    }

    pub fn subject(
        &mut self,
        span: Span,
        instance: Instance,
        permission: Option<Span>,
    ) -> SubjectId {
        self.subjects.push(span, instance, permission)
    }

    pub fn finish(self) -> Ast<'src> {
        Ast {
            types: self.types.finish(),
            lets: self.lets.finish(),
            defs: self.defs.finish(),
            exprs: self.exprs.finish(),
            tests: self.tests.finish(),
            relations: self.relations.finish(),
            subjects: self.subjects.finish(),
            asserts: self.asserts.finish(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Instance {
    ty: Span,
    ident: Span,
}

impl Instance {
    pub fn new(ty: Span, ident: Span) -> Self {
        Self { ty, ident }
    }

    #[inline]
    pub const fn ty(&self) -> Span {
        self.ty
    }

    #[inline]
    pub const fn ident(&self) -> Span {
        self.ident
    }
}
