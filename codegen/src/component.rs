use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentStorage {
    /// Backed by a vector: this means that data is reserved
    /// for every entity, including ones that don't have it, this
    /// trades memory for faster iteration and search.
    Vec,

    /// Backed by a HashMap: efficient memory use and fairly fast
    /// iteration and search.
    /// /!\ **NOTE**: this uses a fast but **non-secure** hashing algorithm.
    HashMap,

    /// Backed by a BTreeMap
    BTreeMap,

    /// Uses a table to map entities and components allowing better memory efficiency.
    DenseVec,

    /// Used for component that do not contain any data (**must implement [`Default`]**)
    Null,

    /// A storage that is flagged for writes: allows detection that the storage has been written to.
    Flagged(Box<Self>),
}

impl ComponentStorage {
    pub fn storage_type(&self, path: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { ::std::vec::Vec<#path> },
            ComponentStorage::HashMap => {
                quote::quote! { ::secs::fxhash::FxHashMap<::secs::Entity, #path> }
            }
            ComponentStorage::BTreeMap => {
                quote::quote! { ::std::collections::BTreeMap<::secs::Entity, #path> }
            }
            ComponentStorage::DenseVec => todo!(),
            ComponentStorage::Null => quote::quote! { () },
            ComponentStorage::Flagged(_) => todo!(),
        }
    }

    pub fn storage_init(&self) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { Vec::new() },
            ComponentStorage::HashMap => quote::quote! { ::secs::fxhash::FxHashMap::new() },
            ComponentStorage::BTreeMap => quote::quote! { ::std::collections::BTreeMap::new() },
            ComponentStorage::DenseVec => todo!(),
            ComponentStorage::Null => quote::quote! { () },
            ComponentStorage::Flagged(_) => todo!(),
        }
    }

    pub fn storage_init_with_capacity(&self, capacity: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { Vec::with_capacity(#capacity) },
            ComponentStorage::HashMap => {
                quote::quote! { ::secs::fxhash::FxHashMap::with_capacity(#capacity) }
            }
            ComponentStorage::BTreeMap => {
                quote::quote! { ::std::collections::BTreeMap::new() }
            }
            ComponentStorage::DenseVec => todo!(),
            ComponentStorage::Null => quote::quote! { () },
            ComponentStorage::Flagged(_) => todo!(),
        }
    }

    pub fn read_function(
        &self,
        component: &Component,
        id: TokenStream,
        store: TokenStream,
        value: TokenStream,
        mutable: bool,
        optional: bool,
    ) -> TokenStream {
        let out = match self {
            ComponentStorage::Vec => {
                if mutable {
                    quote::quote! { #value.get_mut(#id.index() as usize).unwrap().as_mut() }
                } else {
                    quote::quote! { #value.get(#id.index() as usize).unwrap().as_ref() }
                }
            }
            ComponentStorage::HashMap | ComponentStorage::BTreeMap => {
                if mutable {
                    quote::quote! { #value.get_mut(&#id) }
                } else {
                    quote::quote! { #value.get(&#id) }
                }
            }
            ComponentStorage::DenseVec => todo!(),
            ComponentStorage::Null => {
                let bitset = component.as_bitset();
                quote::quote! { if #store.#bitset.contains(#id.index()) { Some(Default::default()) } else { None } }
            }
            ComponentStorage::Flagged(_) => todo!(),
        };

        if optional {
            out
        } else {
            quote::quote! { #out.unwrap() }
        }
    }

    pub fn write_function(
        &self,
        path: TokenStream,
        id: TokenStream,
        value: TokenStream,
    ) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! {
                if #path.len() <= #id.index() as usize {
                    #path.resize(#id.index() as usize + 1, None);
                }

                #path[#id.index() as usize] = Some(#value);
            },
            ComponentStorage::HashMap | ComponentStorage::BTreeMap => {
                quote::quote! { #path.insert(#id, #value); }
            }
            ComponentStorage::Null => quote::quote! {},
            ComponentStorage::DenseVec => todo!(),
            ComponentStorage::Flagged(_) => todo!(),
        }
    }

    pub fn remove_function(
        &self,
        component: &Component,
        path: TokenStream,
        id: TokenStream,
        exists: TokenStream,
    ) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { #path[#id.index() as usize].take() },
            ComponentStorage::HashMap | ComponentStorage::BTreeMap => {
                quote::quote! { #path.remove(&#id) }
            }
            ComponentStorage::Null => {
                let ty = component.as_ty();
                quote::quote! {
                    if #exists {
                        Some(<#ty>::default())
                    } else {
                        None
                    }
                }
            }
            ComponentStorage::Flagged(flagged_inner) => {
                flagged_inner.remove_function(component, path, id, exists)
            }
            _ => quote::quote! {},
        }
    }

    pub fn clear_function(
        &self,
        caller: TokenStream,
        bitset: TokenStream,
        id: TokenStream,
    ) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! {
                if (#id.index() as usize) <= #caller.len() {
                    #caller[#id.index() as usize] = None;
                    #bitset.remove(#id.index());
                } else {
                    #caller.resize(#id.index() as usize + 1, None)
                }
            },
            ComponentStorage::BTreeMap | ComponentStorage::HashMap => quote::quote! {
                #caller.remove(&#id);
                #bitset.remove(#id.index());
            },
            ComponentStorage::Null => quote::quote! {
                #bitset.remove(#id.index());
            },
            ComponentStorage::Flagged(flagged_inner) => {
                flagged_inner.clear_function(caller, bitset, id)
            }
            ComponentStorage::DenseVec => todo!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// The path to the component
    pub path: String,

    /// The name of the component (allows multiple components with the same type but different names)
    pub name: String,

    /// The type of storage used by this component
    pub storage: ComponentStorage,

    /// List of lifetimes the `path` contains
    pub lifetimes: Option<Vec<String>>,
}

impl ComponentStorage {
    pub fn as_type(&self, comp: &Component, ty: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { Vec<Option<#ty>> },
            ComponentStorage::HashMap => {
                quote::quote! { ::fxhash::FxHashMap<::secs::Entity, #ty> }
            }
            ComponentStorage::BTreeMap => {
                quote::quote! { ::std::collections::BTreeMap<::secs::Entity, #ty> }
            }
            ComponentStorage::DenseVec => quote::quote! { ::secs::DenseVec<#ty> },
            ComponentStorage::Null => {
                if let Some(lifetimes) = &comp.lifetimes {
                    if !lifetimes.is_empty() {
                        panic!("Null components cannot have lifetimes, found for: {}", comp.name);
                    }
                }
                quote::quote! { () }
            },
            ComponentStorage::Flagged(flagged) => {
                let flagged_ty = flagged.as_type(comp, ty);
                quote::quote! { ::secs::Flagged<#flagged_ty> }
            }
        }
    }
}

impl Component {
    pub fn as_field_name(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn as_mut_name(&self) -> String {
        format!("{}_mut", self.name).to_case(Case::Snake)
    }

    pub fn as_bitset_name(&self) -> String {
        format!("bitset_{}_", self.name).to_case(Case::Snake)
    }

    pub fn as_del_name(&self) -> String {
        format!("del_{}", self.name).to_case(Case::Snake)
    }

    pub fn as_add_name(&self) -> String {
        format!("add_{}", self.name).to_case(Case::Snake)
    }

    pub fn as_ident(&self) -> Ident {
        Ident::new(&self.as_field_name(), Span::call_site())
    }

    pub fn as_mut(&self) -> Ident {
        Ident::new(&self.as_mut_name(), Span::call_site())
    }

    pub fn as_add_ident(&self) -> Ident {
        Ident::new(&self.as_add_name(), Span::call_site())
    }

    pub fn as_del_ident(&self) -> Ident {
        Ident::new(&self.as_del_name(), Span::call_site())
    }

    pub fn as_bitset(&self) -> Ident {
        Ident::new(&self.as_bitset_name(), Span::call_site())
    }

    pub fn as_ty(&self) -> TokenStream {
        syn::parse_str(&self.path).expect("Failed to parse path")
    }

    pub fn as_storage(&self) -> TokenStream {
        let ty = self.as_ty();
        self.storage.as_type(self, ty)
    }

    pub fn as_struct_field(&self) -> TokenStream {
        let name = self.as_ident();
        let storage = self.as_storage();

        quote::quote! {
            #name: #storage
        }
    }

    pub fn as_struct_bitset(&self) -> TokenStream {
        let name = self.as_bitset();

        quote::quote! {
            #name: ::secs::hibitset::BitSet
        }
    }

    pub fn as_field_ref(&self, mutable: bool) -> TokenStream {
        let name = self.as_ident();
        if mutable {
            quote::quote! {
                &#name
            }
        } else {
            quote::quote! {
                &mut #name
            }
        }
    }
}
