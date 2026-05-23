use aes_ast::TestDefId;

use crate::Visitor;

pub fn walk_test_def<'src>(visit: &mut impl Visitor<'src>, id: TestDefId) {
    let test = visit.ast().tests().at(id);
    let relations = test.relations();
    let asserts = test.asserts();

    for relation_id in relations.iter() {
        visit.relation(relation_id);
    }

    for assert_id in asserts.iter() {
        visit.assert(assert_id);
    }
}
