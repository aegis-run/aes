use aes_allocator::Allocator;
use aes_ast::{AssertionKind, AstBuilder, BinaryOp, ExprTerm, Instance};
use aes_testing::SPAN;

use super::*;

#[derive(Debug, PartialEq, Clone, Copy)]
enum Evt {
    TypeDef,
    LetMember,
    DefMember,
    Expr,
    ExprSelfRef,
    ExprTraversal,
    ExprTypeRef,
    ExprUsersetTypeRef,
    ExprBinary,
    TestDef,
    Relation,
    Assert,
}

#[derive(Debug)]
struct Recorder<'src> {
    ast: &'src Ast<'src>,
    evts: Vec<Evt>,
}

impl<'src> Visitor<'src> for Recorder<'src> {
    fn ast(&self) -> &'src Ast<'src> {
        self.ast
    }

    fn type_def(&mut self, id: TypeDefId) {
        self.evts.push(Evt::TypeDef);
        walk_type_def(self, id);
    }

    fn let_member(&mut self, id: LetMemberId) {
        self.evts.push(Evt::LetMember);
        walk_let_member(self, id);
    }

    fn def_member(&mut self, id: DefMemberId) {
        self.evts.push(Evt::DefMember);
        walk_def_member(self, id);
    }

    fn expr(&mut self, id: ExprId) {
        self.evts.push(Evt::Expr);
        walk_expr(self, id);
    }

    fn expr_self_ref(&mut self, _: ExprTermSelfRef) {
        self.evts.push(Evt::ExprSelfRef);
    }

    fn expr_traversal(&mut self, _: ExprTermTraversal) {
        self.evts.push(Evt::ExprTraversal);
    }

    fn expr_type_ref(&mut self, _: ExprTermTypeRef) {
        self.evts.push(Evt::ExprTypeRef);
    }

    fn expr_userset_type_ref(&mut self, _: ExprTermUsersetTypeRef) {
        self.evts.push(Evt::ExprUsersetTypeRef);
    }

    fn expr_binary(&mut self, _: ExprTermBinary) {
        self.evts.push(Evt::ExprBinary);
    }

    fn test_def(&mut self, id: TestDefId) {
        self.evts.push(Evt::TestDef);
        walk_test_def(self, id);
    }

    fn relation(&mut self, _: RelationId) {
        self.evts.push(Evt::Relation);
    }

    fn assert(&mut self, _: AssertId) {
        self.evts.push(Evt::Assert);
    }
}

fn run(build: impl Fn(&mut AstBuilder)) -> Vec<Evt> {
    let alloc = Allocator::new();
    let ast = aes_testing::ast::build_ast(&alloc, build);

    let mut recorder = Recorder {
        ast: &ast,
        evts: Vec::new(),
    };
    schema(&mut recorder);
    recorder.evts
}

#[test]
fn empty_schema_visits_nothing() {
    let evts = run(|_| {});

    assert!(evts.is_empty());
}

#[test]
fn empty_type_visits_only_type_def() {
    let evts = run(|b| {
        b.type_def(SPAN, SPAN, b.lets.empty_range(), b.defs.empty_range());
    });

    assert_eq!(evts, vec![Evt::TypeDef]);
}

#[test]
fn two_empty_types_visits_two_type_defs() {
    let evts = run(|b| {
        b.type_def(SPAN, SPAN, b.lets.empty_range(), b.defs.empty_range());
        b.type_def(SPAN, SPAN, b.lets.empty_range(), b.defs.empty_range());
    });

    assert_eq!(evts, vec![Evt::TypeDef, Evt::TypeDef]);
}

#[test]
fn let_member_with_type_ref() {
    let evts = run(|b| {
        let expr = b.expr(SPAN, ExprTerm::type_ref(SPAN));
        let lets = {
            let cp = b.lets.checkpoint();
            b.let_member(SPAN, SPAN, expr);
            b.lets.since(cp)
        };
        b.type_def(SPAN, SPAN, lets, b.defs.empty_range());
    });

    assert_eq!(
        evts,
        vec![Evt::TypeDef, Evt::LetMember, Evt::Expr, Evt::ExprTypeRef]
    );
}

#[test]
fn def_member_with_self_ref() {
    let evts = run(|b| {
        let expr = b.expr(SPAN, ExprTerm::self_ref(SPAN));
        let defs = {
            let cp = b.defs.checkpoint();
            b.def_member(SPAN, SPAN, expr);
            b.defs.since(cp)
        };
        b.type_def(SPAN, SPAN, b.lets.empty_range(), defs);
    });

    assert_eq!(
        evts,
        vec![Evt::TypeDef, Evt::DefMember, Evt::Expr, Evt::ExprSelfRef]
    );
}

#[test]
fn def_member_with_traversal() {
    let evts = run(|b| {
        let expr = b.expr(SPAN, ExprTerm::traversal(SPAN, SPAN));
        let defs = {
            let cp = b.defs.checkpoint();
            b.def_member(SPAN, SPAN, expr);
            b.defs.since(cp)
        };
        b.type_def(SPAN, SPAN, b.lets.empty_range(), defs);
    });

    assert_eq!(
        evts,
        vec![Evt::TypeDef, Evt::DefMember, Evt::Expr, Evt::ExprTraversal]
    );
}

#[test]
fn let_member_with_userset_type_ref() {
    let evts = run(|b| {
        let expr = b.expr(SPAN, ExprTerm::userset_type_ref(SPAN, SPAN));
        let lets = {
            let cp = b.lets.checkpoint();
            b.let_member(SPAN, SPAN, expr);
            b.lets.since(cp)
        };
        b.type_def(SPAN, SPAN, lets, b.defs.empty_range());
    });

    assert_eq!(
        evts,
        vec![
            Evt::TypeDef,
            Evt::LetMember,
            Evt::Expr,
            Evt::ExprUsersetTypeRef
        ]
    );
}

#[test]
fn binary_expr_visits_both_sides() {
    let evts = run(|b| {
        let lhs = b.expr(SPAN, ExprTerm::self_ref(SPAN));
        let rhs = b.expr(SPAN, ExprTerm::userset_type_ref(SPAN, SPAN));
        let expr = b.expr(SPAN, ExprTerm::binary(BinaryOp::Union, lhs, rhs));

        let defs = {
            let cp = b.defs.checkpoint();
            b.def_member(SPAN, SPAN, expr);
            b.defs.since(cp)
        };
        b.type_def(SPAN, SPAN, b.lets.empty_range(), defs);
    });

    assert_eq!(
        evts,
        vec![
            Evt::TypeDef,
            Evt::DefMember,
            Evt::Expr,
            Evt::ExprBinary,
            Evt::Expr,
            Evt::ExprSelfRef,
            Evt::Expr,
            Evt::ExprUsersetTypeRef,
        ]
    );
}

#[test]
fn paren_expr_visits_inner() {
    let evts = run(|b| {
        let inner = b.expr(SPAN, ExprTerm::self_ref(SPAN));
        let expr = b.expr(SPAN, ExprTerm::paren(inner));
        let defs = {
            let cp = b.defs.checkpoint();
            b.def_member(SPAN, SPAN, expr);
            b.defs.since(cp)
        };
        b.type_def(SPAN, SPAN, b.lets.empty_range(), defs);
    });

    assert_eq!(
        evts,
        vec![Evt::TypeDef, Evt::DefMember, Evt::Expr, Evt::ExprSelfRef]
    );
}

#[test]
fn multiple_members_visited_in_order() {
    let evts = run(|b| {
        let e1 = b.expr(SPAN, ExprTerm::type_ref(SPAN));
        let e2 = b.expr(SPAN, ExprTerm::type_ref(SPAN));
        let e3 = b.expr(SPAN, ExprTerm::self_ref(SPAN));

        let lets = {
            let cp = b.lets.checkpoint();
            b.let_member(SPAN, SPAN, e1);
            b.let_member(SPAN, SPAN, e2);
            b.lets.since(cp)
        };
        let defs = {
            let cp = b.defs.checkpoint();
            b.def_member(SPAN, SPAN, e3);
            b.defs.since(cp)
        };
        b.type_def(SPAN, SPAN, lets, defs);
    });

    assert_eq!(
        evts,
        vec![
            Evt::TypeDef,
            Evt::LetMember,
            Evt::Expr,
            Evt::ExprTypeRef,
            Evt::LetMember,
            Evt::Expr,
            Evt::ExprTypeRef,
            Evt::DefMember,
            Evt::Expr,
            Evt::ExprSelfRef,
        ]
    );
}

#[test]
fn test_def() {
    let evts = run(|b| {
        let relations = {
            let cp = b.relations.checkpoint();

            let sub = b.subject(SPAN, Instance::new(SPAN, SPAN), None);
            b.relation(SPAN, Instance::new(SPAN, SPAN), SPAN, sub);

            b.relations.since(cp)
        };

        let asserts = {
            let cp = b.asserts.checkpoint();

            b.assert(
                SPAN,
                AssertionKind::Assert,
                Instance::new(SPAN, SPAN),
                SPAN,
                Instance::new(SPAN, SPAN),
            );

            b.asserts.since(cp)
        };

        b.test_def(SPAN, SPAN, relations, asserts);
    });

    assert_eq!(evts, vec![Evt::TestDef, Evt::Relation, Evt::Assert]);
}
