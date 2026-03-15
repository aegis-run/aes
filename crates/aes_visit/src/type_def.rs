use aes_ast::{DefMemberId, LetMemberId, TypeDefId};

use crate::Visitor;

pub fn walk_type_def<'src>(visit: &mut impl Visitor<'src>, id: TypeDefId) {
    let ty = visit.ast().types().at(id);

    for let_ref in visit.ast().lets().range(ty.lets()) {
        visit.let_member(let_ref.id());
    }

    for def_ref in visit.ast().defs().range(ty.defs()) {
        visit.def_member(def_ref.id());
    }
}

pub fn walk_let_member<'src>(visit: &mut impl Visitor<'src>, id: LetMemberId) {
    let eid = visit.ast().lets().at(id).expr();
    visit.expr(eid);
}

pub fn walk_def_member<'src>(visit: &mut impl Visitor<'src>, id: DefMemberId) {
    let eid = visit.ast().defs().at(id).expr();
    visit.expr(eid);
}
