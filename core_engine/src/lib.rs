
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(Component)]
pub fn my_macro_here_derive(input: TokenStream) -> TokenStream { 
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_component_macro(&ast)
}

fn impl_component_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {

        impl<'a> ecs::Component<'a> for #name {
            type RefType = &'a #name;
            type RefMutType = &'a mut #name;
            fn query(world: &'a ecs::World) -> Box<dyn Iterator<Item=Self::RefType> + 'a> {
                let it = world.get::<#name>()
                    .filter_map(|a| {
                        Some( a.as_ref()? )
                    });
        
                Box::new(it)
            }

            fn query_mut(world: &'a mut ecs::World) -> Box<dyn Iterator<Item=Self::RefMutType> + 'a> {
                let it = if let Some(idx) = world.get_index::<#name>() {
                    let mut c = &mut world.components[..];

                    let (_, mid_to_len_c) = c.split_at_mut(idx);
                    let cur_c = &mut mid_to_len_c[0];
                    let cur_c = cur_c
                        .as_any_mut()
                        .downcast_mut::<Vec<Option<#name>>>()
                        .unwrap();
                    cur_c.iter_mut()
                } else {
                    [].iter_mut()
                };

                let it = it
                    .filter_map(|a| {
                        Some( a.as_mut()? )
                    });
        
                Box::new(it)
            }
        }
    };
    gen.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
