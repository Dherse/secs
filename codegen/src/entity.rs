use proc_macro2::TokenStream;

use crate::{GenericOutput, component::Component, ecs::ECS};

pub(crate) fn make_entity_builder(main: &ECS, components: &[Component], generics: &GenericOutput,) -> TokenStream {
    let name = main.as_entity_builder_ident();

    let fields = components.iter().map(|comp| {
        let name = comp.as_ident();
        let ty = comp.as_ty();

        quote::quote! { #name: Option<#ty> }
    });

    let fields_default = components.iter().map(|comp| {
        let name = comp.as_ident();

        quote::quote! { #name: None }
    });

    let setters_fn = components.iter().map(|comp| {
        let name = comp.as_ident();
        let name_add = comp.as_add_ident();
        let name_del = comp.as_del_ident();
        let ty = comp.as_ty();
        let doc_str = format!(
            "Adds the component '{}' of type [`{}`] to the entity",
            comp.name, comp.path
        );
        let doc_del = format!(
            "Removes the component '{}' of type [`{}`] to the entity",
            comp.name, comp.path
        );
        quote::quote! {
            #[doc = #doc_str]
            pub fn #name(mut self, value: #ty) -> Self {
                self.#name = Some(value);
                self
            }

            #[doc = #doc_str]
            pub fn #name_add(&mut self, value: #ty) -> &mut Self {
                self.#name = Some(value);
                self
            }

            #[doc = #doc_del]
            pub fn #name_del(&mut self) -> &mut Self {
                self.#name = None;
                self
            }
        }
    });

    let component_generics = &generics.components;

    quote::quote! {
        pub struct #name#component_generics {
            entity: ::secs::Entity,
            #(#fields,)*
        }

        impl#component_generics #name#component_generics {
            fn new(entity: ::secs::Entity) -> Self {
                Self {
                    entity,
                    #(#fields_default,)*
                }
            }

            pub fn entity(&self) -> ::secs::Entity {
                self.entity
            }

            #(#setters_fn)*
        }
    }
}
