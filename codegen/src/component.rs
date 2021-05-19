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
                quote::quote! { if self.#bitset.contains(#id.index()) { Some(Default::default()) } else { None } }
            }
            ComponentStorage::Flagged(_) => todo!(),
        };

        if optional {
            out
        } else {
            quote::quote! { #out.unwrap() }
        }
    }

    pub fn write_function(&self, id: TokenStream, value: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { [#id.index() as usize] = Some(#value); },
            ComponentStorage::HashMap | ComponentStorage::BTreeMap => {
                quote::quote! { .insert(#id, #value); }
            }
            ComponentStorage::DenseVec => todo!(),
            ComponentStorage::Null => quote::quote! { ; },
            ComponentStorage::Flagged(_) => todo!(),
        }
    }

    pub fn remove_function(
        &self,
        component: &Component,
        id: TokenStream,
        exists: TokenStream,
    ) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { [#id.index() as usize].take() },
            ComponentStorage::HashMap | ComponentStorage::BTreeMap => {
                quote::quote! { .remove(&#id) }
            }
            ComponentStorage::Null => {
                let ty = component.as_ty();
                quote::quote! {
                    ; if #exists {
                        Some(#ty::default())
                    } else {
                        None
                    }
                }
            }
            ComponentStorage::Flagged(flagged_inner) => {
                flagged_inner.remove_function(component, id, exists)
            }
            _ => quote::quote! {},
        }
    }

    pub fn alloc_function(&self, caller: TokenStream, id: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { #caller.push(None); },
            ComponentStorage::Flagged(flagged_inner) => flagged_inner.alloc_function(caller, id),
            _ => quote::quote! {},
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
}

impl ComponentStorage {
    pub fn as_type(&self, ty: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { Vec<Option<#ty>> },
            ComponentStorage::HashMap => {
                quote::quote! { ::fxhash::FxHashMap<::secs::Entity, #ty> }
            }
            ComponentStorage::BTreeMap => {
                quote::quote! { ::std::collections::BTreeMap<::secs::Entity, #ty> }
            }
            ComponentStorage::DenseVec => quote::quote! { ::secs::DenseVec<#ty> },
            ComponentStorage::Null => quote::quote! { () },
            ComponentStorage::Flagged(flagged) => {
                let flagged_ty = flagged.as_type(ty);
                quote::quote! { ::secs::Flagged<#flagged_ty> }
            }
        }
    }

    pub fn as_type_init(&self, ty: TokenStream) -> TokenStream {
        match self {
            ComponentStorage::Vec => quote::quote! { Vec::<Option<#ty>> },
            ComponentStorage::HashMap => {
                quote::quote! { ::fxhash::FxHashMap::<::secs::Entity, #ty> }
            }
            ComponentStorage::BTreeMap => {
                quote::quote! { ::std::collections::BTreeMap::<::secs::Entity, #ty> }
            }
            ComponentStorage::DenseVec => quote::quote! { ::secs::DenseVec::<#ty> },
            ComponentStorage::Null => quote::quote! { ::secs::Null::<#ty> },
            ComponentStorage::Flagged(flagged) => {
                let flagged_ty = flagged.as_type(ty);
                quote::quote! { ::secs::Flagged::<#flagged_ty> }
            }
        }
    }
}

impl Component {
    pub fn as_field_name(&self) -> String {
        format!("comp_{}", self.name.to_case(Case::Snake))
    }

    pub fn as_bitset_name(&self) -> String {
        format!("com_bitset_{}", self.name.to_case(Case::Snake))
    }

    pub fn as_del_name(&self) -> String {
        format!("remove_comp_{}", self.name.to_case(Case::Snake))
    }

    pub fn as_ident(&self) -> Ident {
        Ident::new(&self.as_field_name(), Span::call_site())
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
        self.storage.as_type(ty)
    }

    pub fn as_storage_init(&self) -> TokenStream {
        let ty = self.as_ty();
        self.storage.as_type_init(ty)
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
