use aes_ast::{Ast, BinaryOp, ExprId, ExprTerm};
use aes_foundation::vfs::FileRef;
use aes_ir::v1 as ir;
use aes_visit::Visitor;

pub struct Exporter<'src> {
    ast: &'src Ast<'src>,
    file: FileRef<'src>,

    types: Vec<ir::TypeDefinition>,
    current_type: Option<ir::TypeDefinition>,
}

impl<'src> Exporter<'src> {
    pub fn new(ast: &'src Ast<'src>, file: FileRef<'src>) -> Self {
        Self {
            ast,
            file,
            types: Vec::with_capacity(ast.types().len()),
            current_type: None,
        }
    }

    pub fn export_schema(mut self) -> ir::Schema {
        aes_visit::schema(&mut self);
        ir::Schema::new(self.types)
    }

    fn export_actors(&self, root: ExprId) -> Vec<ir::ActorType> {
        let capacity = match self.ast.exprs().at(root).term() {
            ExprTerm::Binary(expr) if expr.op == BinaryOp::Union => 2,
            _ => 1,
        };

        let mut exporter = ActorExporter {
            ast: self.ast,
            file: self.file,
            actors: Vec::with_capacity(capacity),
        };

        exporter.expr(root);
        exporter.actors
    }

    fn export_expr(&self, root: ExprId) -> Option<ir::Expression> {
        let mut exporter = ExprExporter {
            ast: self.ast,
            file: self.file,
            values: Vec::with_capacity(4),
        };

        aes_visit::walk_expr_postorder(&mut exporter, root);

        let expr = exporter.values.pop().flatten();
        debug_assert!(
            exporter.values.is_empty(),
            "expression exporter left extra values on the stack"
        );

        expr
    }

    #[allow(clippy::unreachable)]
    fn current_type(&mut self) -> &mut ir::TypeDefinition {
        debug_assert!(
            self.current_type.is_some(),
            "member exported outside type_def traversal"
        );

        let Some(type_def) = &mut self.current_type else {
            unreachable!("member exported outside type_def traversal");
        };

        type_def
    }
}

struct ActorExporter<'src> {
    ast: &'src Ast<'src>,
    file: FileRef<'src>,
    actors: Vec<ir::ActorType>,
}

impl<'src> Visitor<'src> for ActorExporter<'src> {
    fn ast(&self) -> &Ast<'src> {
        self.ast
    }

    fn expr_type_ref(&mut self, _: ExprId, expr: aes_ast::ExprTermTypeRef) {
        self.actors
            .push(ir::ActorType::direct(expr.span.text(self.file.source())))
    }

    fn expr_userset_type_ref(&mut self, _: ExprId, expr: aes_ast::ExprTermUsersetTypeRef) {
        self.actors.push(ir::ActorType::userset(
            expr.ty.text(self.file.source()),
            expr.member.text(self.file.source()),
        ))
    }
}

struct ExprExporter<'src> {
    ast: &'src Ast<'src>,
    file: FileRef<'src>,
    values: Vec<Option<ir::Expression>>,
}

impl<'src> ExprExporter<'src> {
    fn push_invalid(&mut self) {
        debug_assert!(
            false,
            "invalid expression reached IR export after semantic validation"
        );
        self.values.push(None);
    }

    fn pop_binary_operands(&mut self) -> Option<(ir::Expression, ir::Expression)> {
        let rhs = self.values.pop().flatten();
        let lhs = self.values.pop().flatten();

        match (lhs, rhs) {
            (Some(lhs), Some(rhs)) => Some((lhs, rhs)),
            _ => {
                self.values.push(None);
                None
            }
        }
    }

    fn same_op_term_count(op: BinaryOp, expr: &ir::Expression) -> usize {
        match (op, &expr.kind) {
            (BinaryOp::Union, Some(ir::expression::Kind::Union(union))) => union.terms.len(),
            (BinaryOp::Intersection, Some(ir::expression::Kind::Intersection(intersection))) => {
                intersection.terms.len()
            }
            _ => 1,
        }
    }

    fn append_same_op_terms(op: BinaryOp, expr: ir::Expression, terms: &mut Vec<ir::Expression>) {
        match (op, expr.kind) {
            (BinaryOp::Union, Some(ir::expression::Kind::Union(mut union))) => {
                terms.append(&mut union.terms);
            }
            (
                BinaryOp::Intersection,
                Some(ir::expression::Kind::Intersection(mut intersection)),
            ) => {
                terms.append(&mut intersection.terms);
            }
            (_, kind) => terms.push(ir::Expression { kind }),
        }
    }

    fn flattened_terms(
        op: BinaryOp,
        lhs: ir::Expression,
        rhs: ir::Expression,
    ) -> Vec<ir::Expression> {
        let capacity = Self::same_op_term_count(op, &lhs) + Self::same_op_term_count(op, &rhs);
        let mut terms = Vec::with_capacity(capacity);

        Self::append_same_op_terms(op, lhs, &mut terms);
        Self::append_same_op_terms(op, rhs, &mut terms);

        terms
    }

    fn binary_expr(op: BinaryOp, lhs: ir::Expression, rhs: ir::Expression) -> ir::Expression {
        match op {
            BinaryOp::Union => ir::Expression::union(Self::flattened_terms(op, lhs, rhs)),
            BinaryOp::Intersection => {
                ir::Expression::intersection(Self::flattened_terms(op, lhs, rhs))
            }
            BinaryOp::Exclusion => ir::Expression::difference(lhs, rhs),
        }
    }
}

impl<'src> Visitor<'src> for ExprExporter<'src> {
    fn ast(&self) -> &Ast<'src> {
        self.ast
    }

    fn expr_type_ref(&mut self, _: ExprId, _: aes_ast::ExprTermTypeRef) {
        self.push_invalid();
    }

    fn expr_userset_type_ref(&mut self, _: ExprId, _: aes_ast::ExprTermUsersetTypeRef) {
        self.push_invalid();
    }

    fn expr_self_ref(&mut self, _: ExprId, expr: aes_ast::ExprTermSelfRef) {
        self.values.push(Some(ir::Expression::self_ref(
            expr.span.text(self.file.source()),
        )));
    }

    fn expr_traversal(&mut self, _: ExprId, expr: aes_ast::ExprTermTraversal) {
        self.values.push(Some(ir::Expression::traversal(
            expr.relation.text(self.file.source()),
            expr.permission.text(self.file.source()),
        )));
    }

    fn expr_binary(&mut self, _: ExprId, expr: aes_ast::ExprTermBinary) {
        let Some((lhs, rhs)) = self.pop_binary_operands() else {
            return;
        };

        self.values.push(Some(Self::binary_expr(expr.op, lhs, rhs)));
    }
}

impl<'src> Visitor<'src> for Exporter<'src> {
    fn ast(&self) -> &Ast<'src> {
        self.ast
    }

    fn type_def(&mut self, id: aes_ast::TypeDefId) {
        let type_ref = self.ast.types().at(id);
        let name = type_ref.name().text(self.file.source()).to_owned();

        self.current_type = Some(ir::TypeDefinition::with_capacity(
            name,
            type_ref.lets().len(),
            type_ref.defs().len(),
        ));

        aes_visit::walk_type_def(self, id);

        if let Some(type_def) = self.current_type.take() {
            self.types.push(type_def);
        }
    }

    fn let_member(&mut self, id: aes_ast::LetMemberId) {
        let let_ref = self.ast.lets().at(id);
        let relation = ir::Relation::new(
            let_ref.name().text(self.file.source()),
            self.export_actors(let_ref.expr()),
        );

        self.current_type().relations.push(relation);
    }

    fn def_member(&mut self, id: aes_ast::DefMemberId) {
        let def_ref = self.ast.defs().at(id);
        let permission = ir::Permission::new(
            def_ref.name().text(self.file.source()),
            self.export_expr(def_ref.expr()),
        );

        self.current_type().permissions.push(permission);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_allocator::Allocator;
    use aes_ir::v1 as ir;
    use indoc::indoc;

    fn export(source: &str) -> ir::Schema {
        let alloc = Allocator::new();
        let file = aes_testing::file_ref(&alloc, source);
        let mut reporter = aes_testing::Reporter::default();

        let ast = aes_parser::Parser::new(file, &mut reporter).parse();
        assert!(
            reporter.is_clean(),
            "parse errors: {:?}",
            reporter.messages()
        );

        let schema = aes_semantic::analyze(file, &ast, &mut reporter);
        assert!(
            schema.is_some(),
            "semantic errors: {:?}",
            reporter.messages()
        );

        Exporter::new(&ast, file).export_schema()
    }

    fn assert_golden(name: &str, schema: &ir::Schema) {
        use prost::Message;
        let actual = schema.encode_to_vec();
        let golden_path = format!("{}/src/testdata/{}.bin", env!("CARGO_MANIFEST_DIR"), name,);

        if std::env::var("GOLDEN_UPDATE").is_ok() {
            if let Some(parent) = std::path::Path::new(&golden_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&golden_path, &actual).expect("failed to write golden file");
            return;
        }

        let expected = std::fs::read(&golden_path).unwrap_or_else(|_| {
            panic!("golden file not found: {golden_path}. Run with GOLDEN_UPDATE=1 to create.")
        });
        assert_eq!(actual, expected, "protobuf encoding mismatch for {name}");
    }

    mod actors {
        use super::*;

        #[test]
        fn single_direct_actor() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let owner = user;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].relations[0].actors);
        }

        #[test]
        fn single_userset_actor() {
            let source = indoc! {r#"
                type team {
                    let member = team;
                }

                type doc {
                    let editor = team::member;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].relations[0].actors);
        }

        #[test]
        fn union_of_direct_actors() {
            let source = indoc! {r#"
                type user {}

                type group {}

                type doc {
                    let reader = user | group;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[2].relations[0].actors);
        }

        #[test]
        fn union_of_mixed_actors() {
            let source = indoc! {r#"
                type user {}

                type team {
                    let member = team;
                }

                type doc {
                    let reader = user | team::member;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[2].relations[0].actors);
        }

        #[test]
        fn three_way_union() {
            let source = indoc! {r#"
                type user {}

                type team {
                    let member = team;
                }

                type org {
                    let admin = org;
                }

                type doc {
                    let reader = user | team::member | org::admin;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[3].relations[0].actors);
        }
    }

    mod expressions {
        use super::*;

        #[test]
        fn self_ref() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let owner = user;
                    def read = .owner;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn traversal() {
            let source = indoc! {r#"
                type user {}

                type group {
                    let member = user;
                    def read = .member;
                }

                type doc {
                    let parent = group;
                    def read = .parent.read;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[2].permissions[0].expr);
        }

        #[test]
        fn binary_union() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let owner = user;
                    let editor = user;
                    def read = .owner | .editor;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn binary_intersection() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let approved = user;
                    let active = user;
                    def member = .approved & .active;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn binary_exclusion() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let all = user;
                    let banned = user;
                    def member = .all - .banned;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn nary_union_flattening() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let a = user;
                    let b = user;
                    let c = user;
                    def perm = .a | .b | .c;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn nary_intersection_flattening() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let a = user;
                    let b = user;
                    let c = user;
                    def perm = .a & .b & .c;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn exclusion_not_flattened() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let a = user;
                    let b = user;
                    let c = user;
                    def perm = .a - .b - .c;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn mixed_operators_precedence() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let a = user;
                    let b = user;
                    let c = user;
                    def perm = .a | .b & .c;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }

        #[test]
        fn parenthesized_grouping() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let a = user;
                    let b = user;
                    let c = user;
                    def perm = (.a | .b) & .c;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema.types[1].permissions[0].expr);
        }
    }

    mod type_defs {
        use super::*;

        #[test]
        fn empty_type() {
            let source = indoc! {r#"
                type user {}
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema);
        }

        #[test]
        fn type_with_relation_only() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let owner = user;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema);
        }

        #[test]
        fn type_with_permission_only() {
            let source = indoc! {r#"
                type user {}

                type doc {
                    let owner = user;
                    def read = .owner;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema);
        }

        #[test]
        fn multiple_types() {
            let source = indoc! {r#"
                type user {}

                type group {
                    let member = user;
                }

                type doc {
                    let owner = group;
                    def read = .owner.member;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema);
        }

        #[test]
        fn full_github_sample() {
            let source = indoc! {r#"
                type user {}

                type team {
                    let parent = organization | team;
                    let maintainer = user;
                    let direct_member = user;

                    def member = .maintainer | .direct_member;
                    def change_team_name = .maintainer | .parent.change_team_name;
                }

                type organization {
                    let owner = user;
                    let member = user;
                    let billing_manager = user;
                    let team_maintainer = user;

                    def create_repository = .owner | .member;
                    def manage_billing = .owner | .billing_manager;
                    def user_seat = .owner | .member | .team_maintainer;
                    def change_team_name = .team_maintainer | .owner;
                }

                type repository {
                    let organization = organization;

                    let reader = user | team::direct_member;
                    let triager = user | team::direct_member;
                    let writer = user | team::direct_member;
                    let maintainer = user | team::direct_member;
                    let admin = user | team::direct_member;

                    def push = .writer | .maintainer | .admin | .organization.owner;
                    def clone = .reader | .triager | .push;
                    def read = .clone | .organization.owner;
                    def delete = .admin | .organization.owner;

                    def create_issue = .read;
                    def close_issue = .triager | .writer | .maintainer | .admin | .organization.owner;

                    def create_pull_request = .read;
                    def merge_pull_request = .maintainer | .organization.owner;
                    def close_pull_request = .close_issue;

                    def manage_setting = .maintainer | .admin | .organization.owner;
                    def manage_sensitive_setting = .admin | .organization.owner;
                }
            "#};
            let schema = export(source);
            insta::assert_debug_snapshot!(schema);
        }
    }

    mod golden {
        use super::*;

        #[test]
        fn minimal_schema() {
            let source = indoc! {r#"
                type group {}

                type user {
                  let owner = group;
                  def read = .owner;
                }
            "#};
            let schema = export(source);
            assert_golden("minimal_schema", &schema);
        }

        #[test]
        fn full_github_sample() {
            let source = indoc! {r#"
                type user {}

                type team {
                    let parent = organization | team;
                    let maintainer = user;
                    let direct_member = user;

                    def member = .maintainer | .direct_member;
                    def change_team_name = .maintainer | .parent.change_team_name;
                }

                type organization {
                    let owner = user;
                    let member = user;
                    let billing_manager = user;
                    let team_maintainer = user;

                    def create_repository = .owner | .member;
                    def manage_billing = .owner | .billing_manager;
                    def user_seat = .owner | .member | .team_maintainer;
                    def change_team_name = .team_maintainer | .owner;
                }

                type repository {
                    let organization = organization;

                    let reader = user | team::direct_member;
                    let triager = user | team::direct_member;
                    let writer = user | team::direct_member;
                    let maintainer = user | team::direct_member;
                    let admin = user | team::direct_member;

                    def push = .writer | .maintainer | .admin | .organization.owner;
                    def clone = .reader | .triager | .push;
                    def read = .clone | .organization.owner;
                    def delete = .admin | .organization.owner;

                    def create_issue = .read;
                    def close_issue = .triager | .writer | .maintainer | .admin | .organization.owner;

                    def create_pull_request = .read;
                    def merge_pull_request = .maintainer | .organization.owner;
                    def close_pull_request = .close_issue;

                    def manage_setting = .maintainer | .admin | .organization.owner;
                    def manage_sensitive_setting = .admin | .organization.owner;
                }
            "#};
            let schema = export(source);
            assert_golden("full_github_sample", &schema);
        }
    }

    /// Generates a semantically valid program (types only, no tests since semantic analyzer currently doesn't process tests in export).
    pub fn semantic_program() -> impl proptest::strategy::Strategy<Value = String> {
        use aes_testing::generate::*;
        use proptest::prelude::*;

        proptest::collection::vec(type_def(), 0..4)
            .prop_map(|types| types.join("\n\n"))
            .prop_filter("must be semantically valid", |s| {
                let alloc = Allocator::new();
                let file = aes_testing::file_ref(&alloc, s);
                let mut reporter = aes_testing::Reporter::default();
                let ast = aes_parser::Parser::new(file, &mut reporter).parse();
                if !reporter.is_clean() {
                    return false;
                }
                aes_semantic::analyze(file, &ast, &mut reporter).is_some()
            })
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn export_never_panics(ref source in semantic_program()) {
                let res = std::panic::catch_unwind(|| {
                    let _ = export(source);
                });

                prop_assert!(res.is_ok());
            }

            #[test]
            fn ir_type_count_matches_ast(ref source in semantic_program()) {
                let alloc = Allocator::new();
                let file = aes_testing::file_ref(&alloc, source);
                let mut reporter = aes_testing::Reporter::default();
                let ast = aes_parser::Parser::new(file, &mut reporter).parse();
                let schema = export(source);

                prop_assert_eq!(ast.types().len(), schema.types.len());
            }

            #[test]
            fn all_expressions_have_kind(ref source in semantic_program()) {
                let schema = export(source);

                fn check_expr(expr: &ir::Expression) -> Result<(), proptest::test_runner::TestCaseError> {
                    prop_assert!(expr.kind.is_some());
                    match expr.kind.as_ref().unwrap() {
                        ir::expression::Kind::Union(u) => {
                            for e in &u.terms { check_expr(e)?; }
                        },
                        ir::expression::Kind::Intersection(i) => {
                            for e in &i.terms { check_expr(e)?; }
                        },
                        ir::expression::Kind::Difference(d) => {
                            if let Some(lhs) = &d.lhs { check_expr(lhs)?; }
                            if let Some(rhs) = &d.rhs { check_expr(rhs)?; }
                        },
                        ir::expression::Kind::Term(_) => {}
                    }
                    Ok(())
                }

                for t in schema.types {
                    for p in t.permissions {
                        if let Some(expr) = p.expr {
                            check_expr(&expr)?;
                        }
                    }
                }
            }

            #[test]
            fn all_relations_have_actors(ref source in semantic_program()) {
                let schema = export(source);
                for t in schema.types {
                    for r in t.relations {
                        prop_assert!(!r.actors.is_empty());
                    }
                }
            }

            #[test]
            fn type_names_are_nonempty(ref source in semantic_program()) {
                let schema = export(source);
                for t in schema.types {
                    prop_assert!(!t.name.is_empty());
                }
            }
        }
    }
}
