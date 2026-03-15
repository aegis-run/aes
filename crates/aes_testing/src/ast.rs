use aes_ast::{Ast, AstBuilder};

pub fn build_ast(alloc: &aes_allocator::Allocator, build: impl Fn(&mut AstBuilder)) -> Ast<'_> {
    let mut b = AstBuilder::new(alloc);
    build(&mut b);
    b.finish()
}
