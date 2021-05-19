use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use serde::{Deserialize, Serialize};
use syn::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The path to the resource type
    pub path: String,

    /// The name of the resource (allows multiple components with the same type but different names)
    pub name: String,

    /// Whether or not the resource implement default
    pub default: bool,
}

impl Resource {
    pub fn as_field_name(&self) -> String {
        format!("resource_{}", self.name.to_case(Case::Snake))
    }

    pub fn as_field_ident(&self) -> Ident {
        Ident::new(&self.as_field_name(), Span::call_site())
    }

    pub fn as_ty(&self) -> Path {
        syn::parse_str(&self.path).expect("Failed to parse path")
    }

    pub fn as_struct_field(&self) -> TokenStream {
        let name = self.as_field_ident();
        let ty: Path = self.as_ty();

        quote::quote! {
            #name: #ty
        }
    }

    pub fn as_builder_field(&self) -> TokenStream {
        let name = self.as_field_ident();
        let ty: Path = self.as_ty();

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