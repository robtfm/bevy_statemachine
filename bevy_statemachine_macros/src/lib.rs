extern crate proc_macro;

use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::{parse_macro_input};

#[proc_macro]
pub fn exclusive_state(input: TokenStream) -> TokenStream {
    let data = parse_macro_input!(input as syn::ItemEnum);

    let attrs = &data.attrs;
    let vis = &data.vis;
    if matches!(vis, syn::Visibility::Inherited) {

    }
    let name = &data.ident;
    let var_names: Vec<_> = data.variants.iter().map(|v| &v.ident).collect();

    let variants = data.variants.iter().map(|v| {
        let attrs = attrs.iter();
        let name = &v.ident;
        let other_variants: Vec<_> = data.variants.iter().filter(|&other| other != v).map(|v| &v.ident).collect();

        quote! {
            #( #attrs )*
            pub(super) struct #v;

            impl super::ExclusiveState for #name {
                type WithoutState = ( 
                    #( bevy::ecs::query::Without<#other_variants>, )*
                );

                fn set_exclusive_state<'a, 'b, 'c, 'd>(self, commands: &'a mut bevy::ecs::system::EntityCommands<'b, 'c, 'd>) -> &'a mut bevy::ecs::system::EntityCommands<'b, 'c, 'd> {
                    commands.insert(self)
                        #( .remove::<#other_variants>() )*
                }
            }
        }        
    });

    let with_name = format_ident!("With{}", name);

    let res = quote! {
        #[allow(non_snake_case)]
        #vis mod #name {
            use super::*;

            #( #variants )*

            pub(super) fn set_sparse(world: &mut bevy::ecs::world::World) {
                #(
                    world.register_component(bevy::ecs::component::ComponentDescriptor::new::<#var_names>(bevy::ecs::component::StorageType::SparseSet)).unwrap();
                )*
            }
        }

        #vis struct #with_name;

        impl bevy::ecs::query::WorldQuery for #with_name {
            type Fetch = <bevy::ecs::query::Or<( 
                #( bevy::ecs::query::With<#name::#var_names>, )*
            )> as bevy::ecs::query::WorldQuery>::Fetch;

            type State = <bevy::ecs::query::Or<( 
                #( bevy::ecs::query::With<#name::#var_names>, )*
            )> as bevy::ecs::query::WorldQuery>::State;
        }
    };

    TokenStream::from(res)
}