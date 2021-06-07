use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource<'a> {
    /// The path to the resource type
    pub path: &'a str,

    /// The name of the resource (allows multiple components with the same type but different names)
    pub name: &'a str,

    /// Whether or not the resource implement default
    pub default: bool,

    /// List of lifetimes the `path` contains
    pub lifetimes: Option<Vec<&'a str>>,
}

impl<'a> Resource<'a> {
    pub fn as_field_name(&self) -> String {
        format!("resource_{}", self.name).to_case(Case::Snake)
    }

    pub fn as_field_ident(&self) -> Ident {
        Ident::new(&self.as_field_name(), Span::call_site())
    }

    pub fn as_ty(&self) -> TokenStream {
        syn::parse_str(&self.path).expect("Failed to parse path")
    }

    pub fn as_struct_field(&self) -> TokenStream {
        let name = self.as_field_ident();
        let ty: TokenStream = self.as_ty();

        quote::quote! {
            #name: #ty
        }
    }

    pub fn as_builder_field(&self) -> TokenStream {
        let name = self.as_field_ident();
        let ty: TokenStream = self.as_ty();

        if self.default {
            quote::quote! {
                #name: #ty
            }
        } else {
            quote::quote! {
                #name: Option<#ty>
            }
        }
    }

    pub fn as_field_ref(&self, mutable: bool) -> TokenStream {
        let name = Ident::new(&self.as_field_name(), Span::call_site());
        if mutable {
            quote::quote! {
                &mut self.#name
            }
        } else {
            quote::quote! {
                &self.#name
            }
        }
    }
}
