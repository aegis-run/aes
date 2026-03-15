use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, ItemStruct, parse_macro_input};

/// Generates structural boilerplate for an Arena/Structure-of-Arrays (SoA) AST node.
///
/// This macro expects a `struct` definition with named fields. All fields must have a type
/// that can be represented as a slice `&[T]` in the memory pool.
///
/// For a given node `Name`, this macro will automatically generate:
///
/// * **`NameId`**: A strongly-typed `u32` identifier (`aes_foundation::Id<Name>`).
/// * **`NameRange`**: A range of contiguous IDs (`aes_foundation::Range<Name>`).
/// * **`NameRef`**: A reference handle providing cheap `O(1)` access to the fields via its `Id`.
/// * **`NamePool`**: A read-only SoA pool storing all nodes of this type as contiguous slices.
/// * **`NamePoolBuilder`**: A mutable builder used during parsing to append nodes into `Vec`s.
///
/// # Example
/// ```ignore
/// #[ast_node]
/// pub struct LetMember {
///     span: Span,
///     name: Span,
///     expr: ExprId,
/// }
/// ```
#[proc_macro_attribute]
pub fn ast_node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let node_def = AstNodeDef::from_ast(&input);

    let types_tokens = generate_types(&node_def);
    let pool_tokens = generate_pool(&node_def);
    let builder_tokens = generate_builder(&node_def);

    let vis = &node_def.vis;
    let name = &node_def.name;

    let expanded = quote! {
        // We emit the original empty struct so it serves as the marker type for Id<T>.
        // We don't emit the fields themselves on this struct since they live in the Pool.
        #vis struct #name;

        #types_tokens
        #pool_tokens
        #builder_tokens
    };

    TokenStream::from(expanded)
}

/// A parsed representation of an AST node struct for macro generation.
struct AstNodeDef<'a> {
    vis: &'a syn::Visibility,
    name: &'a syn::Ident,
    field_names: Vec<&'a syn::Ident>,
    field_tys: Vec<&'a syn::Type>,
}

impl<'a> AstNodeDef<'a> {
    fn from_ast(input: &'a ItemStruct) -> Self {
        let fields = match &input.fields {
            Fields::Named(f) => f.named.iter().collect::<Vec<_>>(),
            _ => panic!("#[ast_node] requires a struct with named fields"),
        };

        if fields.is_empty() {
            panic!("#[ast_node] structs must have at least one field");
        }

        Self {
            vis: &input.vis,
            name: &input.ident,
            field_names: fields.iter().map(|f| f.ident.as_ref().unwrap()).collect(),
            field_tys: fields.iter().map(|f| &f.ty).collect(),
        }
    }

    fn first_field(&self) -> &syn::Ident {
        self.field_names[0]
    }

    fn id_name(&self) -> syn::Ident {
        format_ident!("{}Id", self.name)
    }
    fn range_name(&self) -> syn::Ident {
        format_ident!("{}Range", self.name)
    }
    fn ref_name(&self) -> syn::Ident {
        format_ident!("{}Ref", self.name)
    }
    fn pool_name(&self) -> syn::Ident {
        format_ident!("{}Pool", self.name)
    }
    fn builder_name(&self) -> syn::Ident {
        format_ident!("{}PoolBuilder", self.name)
    }
}

fn generate_types(node: &AstNodeDef) -> impl quote::ToTokens {
    let vis = node.vis;
    let name = node.name;
    let id_name = node.id_name();
    let range_name = node.range_name();
    let ref_name = node.ref_name();
    let pool_name = node.pool_name();

    let id_doc = format!("A unique `u32` identifier for a [`{name}`] node in the AST.");
    let range_doc = format!("A contiguous range of [`{name}`] node IDs.");
    let ref_doc = format!("A lightweight, `O(1)` reference handle to a [`{name}`] node.");

    let ref_getters = node
        .field_names
        .iter()
        .zip(node.field_tys.iter())
        .map(|(n, t)| {
            quote! {
                #[inline]
                pub const fn #n(&self) -> #t {
                    self.pool.#n[self.id.as_index()]
                }
            }
        });

    quote! {
        #[doc = #id_doc]
        #vis type #id_name = aes_foundation::Id<#name>;

        #[doc = #range_doc]
        #vis type #range_name = aes_foundation::Range<#name>;

        #[doc = #ref_doc]
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
    }
}

fn generate_pool(node: &AstNodeDef) -> impl quote::ToTokens {
    let vis = node.vis;
    let pool_name = node.pool_name();
    let id_name = node.id_name();
    let range_name = node.range_name();
    let ref_name = node.ref_name();

    let field_names = &node.field_names;
    let field_tys = &node.field_tys;
    let first_field = node.first_field();

    let pool_doc = format!(
        "A memory-efficient SoA storage pool for all [`{}`] nodes.",
        node.name
    );

    let len_asserts = field_names.windows(2).map(|w| {
        let (a, b) = (w[0], w[1]);
        quote! { debug_assert!(#a.len() == #b.len()); }
    });

    quote! {
        #[doc = #pool_doc]
        #[derive(Debug, Clone, Copy)]
        #vis struct #pool_name<'src> {
            #( #field_names: &'src [#field_tys], )*
        }

        impl<'src> #pool_name<'src> {
            pub fn new( #( #field_names: &'src [#field_tys] ),* ) -> Self {
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
    }
}

fn generate_builder(node: &AstNodeDef) -> impl quote::ToTokens {
    let vis = node.vis;
    let name = node.name;
    let id_name = node.id_name();
    let range_name = node.range_name();
    let pool_name = node.pool_name();
    let builder_name = node.builder_name();

    let field_names = &node.field_names;
    let field_tys = &node.field_tys;
    let first_field = node.first_field();

    let builder_doc =
        format!("A mutable builder for constructing a [`{pool_name}`] incrementally.");

    quote! {
        #[doc = #builder_doc]
        #[derive(Debug)]
        #vis struct #builder_name<'src> {
            #( pub #field_names: aes_allocator::Vec<'src, #field_tys>, )*
        }

        impl<'src> #builder_name<'src> {
            pub fn new(alloc: &'src aes_allocator::Allocator) -> Self {
                Self { #( #field_names: aes_allocator::Vec::new_in(alloc), )* }
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

            pub fn push(&mut self, #( #field_names: #field_tys, )* ) -> #id_name {
                let id = self.current_id();
                #( self.#field_names.push(#field_names); )*
                id
            }

            pub fn finish(self) -> #pool_name<'src> {
                #pool_name::new( #( self.#field_names.into_bump_slice(), )* )
            }
        }
    }
}
