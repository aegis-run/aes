use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, ItemStruct, parse_macro_input};

#[proc_macro_attribute]
pub fn ast_node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let vis = &input.vis;
    let name = &input.ident;

    let id_name = format_ident!("{name}Id");
    let range_name = format_ident!("{name}Range");
    let pool_name = format_ident!("{name}Pool");
    let ref_name = format_ident!("{name}Ref");
    let builder_name = format_ident!("{name}PoolBuilder");

    let fields = match &input.fields {
        Fields::Named(f) => f.named.iter().collect::<Vec<_>>(),
        _ => panic!("generate_pool requires named fields"),
    };

    let first_field = fields[0].ident.as_ref().expect("named field");

    let field_names = fields
        .iter()
        .map(|f| f.ident.as_ref().unwrap())
        .collect::<Vec<_>>();
    let field_tys = fields.iter().map(|f| &f.ty).collect::<Vec<_>>();

    let pool_fields = field_names.iter().zip(field_tys.iter()).map(|(n, t)| {
        quote! { #n: &'src [#t] }
    });

    let ref_getters = field_names.iter().zip(field_tys.iter()).map(|(n, t)| {
        quote! {
            #[inline]
            pub const fn #n(&self) -> #t {
                self.pool.#n[self.id.as_index()]
            }
        }
    });

    let new_args = field_names.iter().zip(field_tys.iter()).map(|(n, t)| {
        quote! { #n: &'src [#t] }
    });

    let len_asserts = field_names.windows(2).map(|w| {
        let a = w[0];
        let b = w[1];
        quote! { debug_assert!(#a.len() == #b.len()); }
    });

    let builder_fields = field_names.iter().zip(field_tys.iter()).map(|(n, t)| {
        quote! { pub #n: aes_allocator::Vec<'src, #t> }
    });

    let builder_inits = field_names.iter().map(|n| {
        quote! { #n: aes_allocator::Vec::new_in(alloc) }
    });

    let builder_push_args = field_names.iter().zip(field_tys.iter()).map(|(n, t)| {
        quote! { #n: #t }
    });

    let builder_push_body = field_names.iter().map(|n| {
        quote! { self.#n.push(#n); }
    });

    let builder_finish_args = field_names.iter().map(|n| {
        quote! { self.#n.into_bump_slice() }
    });

    let expanded = quote! {
        // #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #vis struct #name;

        #vis type #id_name = aes_foundation::Id<#name>;
        #vis type #range_name = aes_foundation::Range<#name>;

        #[derive(Clone, Copy)]
        #vis struct #ref_name<'src> {
            pool: &'src #pool_name<'src>,
            id:   #id_name,
        }

        impl<'src> #ref_name<'src> {
            #[inline]
            pub const fn id(&self) -> #id_name { self.id }
            #( #ref_getters )*
        }

        #[derive(Debug, Clone, Copy)]
        #vis struct #pool_name<'src> {
            #( #pool_fields, )*
        }

        impl<'src> #pool_name<'src> {
            pub fn new( #( #new_args ),* ) -> Self {
                #( #len_asserts )*
                Self { #( #field_names ),* }
            }

            #[inline]
            pub const fn len(&self) -> usize { self.#first_field.len() }

            #[inline]
            pub const fn is_empty(&self) -> bool { self.#first_field.is_empty() }

            #[inline]
            pub fn at(&self, id: #id_name) -> #ref_name<'_> {
                debug_assert!(
                    id.as_index() < self.len(),
                    "pool index out of bounds: id={} len={}",
                    id.as_index(),
                    self.len(),
                );
                #ref_name { pool: self, id }
            }

            pub fn range(&self, range: #range_name) -> impl Iterator<Item = #ref_name<'_>> {
                range.iter().map(move |id| self.at(id))
            }
        }

        #vis struct #builder_name<'src> {
            #( #builder_fields, )*
        }

        impl<'src> #builder_name<'src> {
            pub fn new(alloc: &'src aes_allocator::Allocator) -> Self {
                Self { #( #builder_inits, )* }
            }

            #[inline]
            fn current_id(&self) -> #id_name {
                #id_name::new(self.#first_field.len() as u32)
            }

            #[inline]
            pub fn checkpoint(&self) -> aes_foundation::Checkpoint<#name> {
                aes_foundation::Checkpoint::new(self.current_id())
            }

            #[inline]
            pub fn since(&self, checkpoint: aes_foundation::Checkpoint<#name>) -> #range_name {
                #range_name::new(checkpoint.id(), self.current_id())
            }

            #[inline]
            pub fn empty_range(&self) -> #range_name {
              let id = self.current_id();
              #range_name::new(id, id)
            }

            pub fn push(&mut self, #( #builder_push_args, )* ) -> #id_name {
                let id = self.current_id();
                #( #builder_push_body )*
                id
            }

            pub fn finish(self) -> #pool_name<'src> {
                #pool_name::new( #( #builder_finish_args, )* )
            }
        }
    };

    TokenStream::from(expanded)
}
