use proc_macro2::TokenStream;

use crate::{ecs::ECS, resource::Resource, system::System, GenericOutput};

pub(crate) fn make_builder(
    main: &ECS,
    resources: &[Resource],
    systems: &[System],
    generics: &GenericOutput,
) -> TokenStream {
    let ecs_name = main.as_ident();
    let name = main.as_builder_ident();
    let store = main.as_component_store_ident();
    let command_buffer = main.as_command_buffer_ident();

    let resource_types: Vec<TokenStream> =
        resources.iter().map(Resource::as_builder_field).collect();

    let system_state_types: Vec<TokenStream> = systems
        .iter()
        .map(System::as_builder_field)
        .flatten()
        .collect();

    let res_functions: Vec<TokenStream> = resources
        .iter()
        .map(|res| {
            let ty = res.as_ty();
            let name = res.as_field_ident();

            let doc_str = format!("Sets the resource '{}' of type [`{}`]", res.name, res.path);
            let value = if res.default {
                quote::quote! { value }
            } else {
                quote::quote! { Some(value) }
            };

            quote::quote! {
                #[doc = #doc_str]
                pub fn #name(mut self, value: #ty) -> Self {
                    self.#name = #value;
                    self
                }
            }
        })
        .collect();

    let res_set: Vec<TokenStream> = resources
        .iter()
        .map(|res| {
            let name = res.as_field_ident();
            if res.default {
                quote::quote! {
                    #name: self.#name
                }
            } else {
                let err_str = format!("Resource `{}` of type `{}` not set", res.name, res.path);
                quote::quote! {
                    #name: self.#name.expect(#err_str)
                }
            }
        })
        .collect();

    let state_functions: Vec<TokenStream> = systems
        .iter()
        .filter_map(|sys| {
            let ty = sys.as_ty()?;
            let name = sys.as_ident();

            let doc_str = format!(
                "Sets state of system '{}' of type `{}`",
                sys.name,
                sys.state.as_ref()?
            );
            Some(quote::quote! {
                #[doc = #doc_str]
                pub fn #name(mut self, value: #ty) -> Self {
                    self.#name = Some(value);
                    self
                }
            })
        })
        .collect();

    let state_set: Vec<TokenStream> = systems
        .iter()
        .filter_map(|sys| {
            let name = sys.as_ident();
            if sys.state.is_some() {
                let err_str = format!(
                    "State of system `{}` of type `{}` not set",
                    sys.name, sys.path
                );
                Some(quote::quote! {
                    #name: self.#name.expect(#err_str)
                })
            } else {
                None
            }
        })
        .collect();

    let ecs_generics = &generics.ecs;
    let builder_generics = &generics.builder;
    let component_generics = &generics.components;

    quote::quote! {
        #[derive(Default)]
        pub struct #name#builder_generics {
            #(#resource_types,)*
            #(#system_state_types,)*
        }

        impl#builder_generics #name#builder_generics {
            #[doc = "Creates a new builder"]
            pub fn new() -> Self {
                Self::default()
            }

            #[doc = "Builds the builder into the ECS"]
            pub fn build#component_generics(self) -> #ecs_name#ecs_generics {
                let components = #store::new();
                #ecs_name {
                    command_buffer: #command_buffer::new(&components),
                    components,
                    #(#res_set,)*
                    #(#state_set,)*
                }
            }

            #[doc = "Builds the builder into the ECS with a capacity"]
            pub fn with_capacity#component_generics(self, capacity: usize) -> #ecs_name#ecs_generics {
                let components = #store::with_capacity(capacity);
                #ecs_name {
                    command_buffer: #command_buffer::new(&components),
                    components,
                    #(#res_set,)*
                    #(#state_set,)*
                }
            }

            #(#res_functions)*

            #(#state_functions)*
        }
    }
}
