use aes_ast::{BinaryOp, ExprTerm};

use crate::{Parser, errors, token::TokenKind};

impl<'src> Parser<'src> {
    pub(crate) fn expr(&mut self) -> aes_ast::ExprId {
        self.expr_bp(0)
    }

    /// Implements Top-Down Operator Precedence (Pratt Parsing) for expressions.
    ///
    /// This resolves the ambiguity of binary operators by comparing their binding power (`bp`).
    /// For example, Intersection (`&`, bp: 3) binds tighter than Union (`|`, bp: 1).
    /// As long as the upcoming operator's binding power is greater than the current minimum `bp`,
    /// it becomes the right-hand side of the tree. Let expressions start with `min_bp = 0`.
    fn expr_bp(&mut self, min_bp: u8) -> aes_ast::ExprId {
        let start = self.start_span();

        let mut lhs = self.term();
        loop {
            let Some((op, bp)) = self.infix_op() else {
                break;
            };
            if bp < min_bp {
                break;
            }

            self.skip();

            let rhs = self.expr_bp(bp + 1);
            lhs = self
                .ast
                .expr(self.end_span(start), ExprTerm::binary(op, lhs, rhs))
        }

        lhs
    }

    fn infix_op(&self) -> Option<(BinaryOp, u8)> {
        match self.token.kind() {
            TokenKind::Pipe => Some((BinaryOp::Union, 1)),
            TokenKind::Amp => Some((BinaryOp::Intersection, 3)),
            TokenKind::Minus => Some((BinaryOp::Exclusion, 5)),
            _ => None,
        }
    }

    fn term(&mut self) -> aes_ast::ExprId {
        let start = self.start_span();

        let term = match self.token.kind() {
            TokenKind::Dot => {
                self.skip();

                let relation = self.ident();
                if self.eat(TokenKind::Dot) {
                    let permission = self.ident();
                    ExprTerm::traversal(relation, permission)
                } else {
                    ExprTerm::self_ref(relation)
                }
            }

            TokenKind::Ident => {
                let ty = self.ident();
                if self.eat(TokenKind::ColonColon) {
                    let member = self.ident();
                    ExprTerm::userset_type_ref(ty, member)
                } else {
                    ExprTerm::type_ref(ty)
                }
            }

            TokenKind::LParen => {
                let inner = self.parenthesized(|p| p.expr());
                ExprTerm::paren(inner)
            }

            _ => {
                self.errors.push(errors::expected_term(self.token));
                return self.ast.expr(self.token.span(), aes_ast::ExprTerm::Err);
            }
        };

        self.ast.expr(self.end_span(start), term)
    }
}

#[cfg(test)]
mod tests {
    use aes_allocator::Allocator;
    use aes_ast::*;

    use crate::parser::tests::parse;

    #[test]
    fn precedence_union_intersection() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = a | b & c; }");
        r.has_no_errors();

        let let_def = r.ast.lets().at(LetMemberId::new(0));
        let root = r.ast.exprs().at(let_def.expr());

        let ExprTerm::Binary(expr) = root.term() else {
            panic!("expected Binary Union, got {:?}", root.term());
        };
        assert_eq!(expr.op, BinaryOp::Union);

        let rhs_expr = r.ast.exprs().at(expr.rhs).term();
        let ExprTerm::Binary(rhs) = rhs_expr else {
            panic!("expected Intersection, got {:?}", rhs_expr);
        };

        assert_eq!(rhs.op, BinaryOp::Intersection);
    }

    #[test]
    fn precedence_exclusion_intersection() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = a & b - c; }");
        r.has_no_errors();

        let let_def = r.ast.lets().at(LetMemberId::new(0));
        let root = r.ast.exprs().at(let_def.expr());

        let ExprTerm::Binary(expr) = root.term() else {
            panic!("expected Binary Intersection, got {:?}", root.term());
        };
        assert_eq!(expr.op, BinaryOp::Intersection);

        let rhs_expr = r.ast.exprs().at(expr.rhs).term();
        let ExprTerm::Binary(rhs) = rhs_expr else {
            panic!("expected Exclusion, got {:?}", rhs_expr);
        };

        assert_eq!(rhs.op, BinaryOp::Exclusion);
    }

    #[test]
    fn parenthesized_override() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = (a | b) & c; }");
        r.has_no_errors();

        let let_def = r.ast.lets().at(LetMemberId::new(0));
        let root = r.ast.exprs().at(let_def.expr());

        let ExprTerm::Binary(expr) = root.term() else {
            panic!("expected Intersection, got {:?}", root.term());
        };
        assert_eq!(expr.op, BinaryOp::Intersection);

        let lhs_expr = r.ast.exprs().at(expr.lhs).term();
        let ExprTerm::Paren(inner) = lhs_expr else {
            panic!("expected Paren, got {:?}", lhs_expr);
        };

        let inner_expr = r.ast.exprs().at(inner.inner).term();
        let ExprTerm::Binary(inner_expr) = inner_expr else {
            panic!("expected Union inside Paren, got {:?}", inner_expr);
        };

        assert_eq!(inner_expr.op, BinaryOp::Union);
    }

    #[test]
    fn paren_node_preserved() {
        let alloc = Allocator::new();
        let r = parse(&alloc, "type t { let x = (user); }");
        r.has_no_errors();

        let let_def = r.ast.lets().at(LetMemberId::new(0));
        let root = r.ast.exprs().at(let_def.expr());
        assert!(matches!(root.term(), ExprTerm::Paren(_)));
    }
}
