use aes_ast::{DefMemberId, LetMemberId, TypeDefId};

use crate::Visitor;

pub fn walk_type_def<'src>(visit: &mut impl Visitor<'src>, id: TypeDefId) {
    let ty = visit.ast().types().at(id);
    let lets = ty.lets();
    let defs = ty.defs();

    for let_id in lets.iter() {
        visit.let_member(let_id);
    }

    for def_id in defs.iter() {
        visit.def_member(def_id);
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
