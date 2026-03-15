use aes_ast::TestDefId;

use crate::Visitor;

pub fn walk_test_def<'src>(visit: &mut impl Visitor<'src>, id: TestDefId) {
    let test = visit.ast().tests().at(id);

    for rel_ref in visit.ast().relations().range(test.relations()) {
        visit.relation(rel_ref.id());
    }

    for assert_ref in visit.ast().asserts().range(test.asserts()) {
        visit.assert(assert_ref.id());
    }
}
