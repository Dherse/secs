use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use serde::{Deserialize, Serialize};
use syn::Path;

use crate::{component::Component, find_component, find_resource, resource::Resource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    /// The name of the system
    pub name: String,

    /// The path of the function
    pub path: String,

    /// The type of the system
    pub kind: SystemKind,

    /// Allows the function to stop the iteration if needed. Only available for [`SystemKind::ForEachFunction`].
    pub control_flow: bool,

    /// Whether this function returns a result, note that the result **must** implements [`Into`] for the ECS' error type.
    pub result: bool,

    /// The stage in which to execute this system
    pub stage: String,

    /// The state of this system, these must be specified when the system starts or implement [`Default`]
    pub state: Option<String>,

    // Signature of the system
    pub signature: Vec<Element>,
}

impl System {
    pub fn as_field_name(&self) -> String {
        format!("system_{}", self.name.to_case(Case::Snake))
    }

    pub fn as_ident(&self) -> Ident {
        Ident::new(&self.as_field_name(), Span::call_site())
    }

    pub fn as_ty(&self) -> Option<Path> {
        Some(syn::parse_str(&self.state.as_ref()?).expect("Failed to parse path"))
    }

    pub fn as_struct_field(&self) -> Option<TokenStream> {
        let name = self.as_ident();
        let ty: Path = self.as_ty()?;

        Some(quote::quote! {
            #name: #ty
        })
    }

    pub fn as_builder_field(&self) -> Option<TokenStream> {
        let name = self.as_ident();
        let ty: Path = self.as_ty()?;

        Some(quote::quote! {
            #name: Option<#ty>
        })
    }

    pub fn as_field_ref(&self, mutable: bool) -> Option<TokenStream> {
        if self.state.is_none() {
            None
        } else {
            let name = self.as_ident();
            Some(if mutable {
                quote::quote! {
                    &mut self.#name,
                }
            } else {
                quote::quote! {
                    &self.#name,
                }
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Accessor {
    /// Access to the named resource is read only
    Read,

    /// Access the named resource through a mutex
    Mutex,

    /// Access the named resource through a read-write lock
    RwLock,

    /// Access to the named resource is read and write
    Write,

    /// Optional component
    Option(Box<Accessor>),
}

impl Accessor {
    pub fn wrapper_init(&self, content: TokenStream, bare: bool) -> TokenStream {
        match self {
            Accessor::Read => {
                if bare {
                    quote::quote! { &#content }
                } else {
                    quote::quote! { #content }
                }
            }
            Accessor::Mutex => quote::quote! { ::secs::parking_lot::Mutex::new(#content) },
            Accessor::RwLock => quote::quote! { ::secs::parking_lot::RwLock::new(#content) },
            Accessor::Write => {
                if bare {
                    quote::quote! { &mut #content }
                } else {
                    quote::quote! { #content }
                }
            }
            Accessor::Option(val) => {
                val.wrapper_init(content, bare)
            }
        }
    }

    pub fn is_mut(&self) -> bool {
        match self {
            Accessor::Read => false,
            Accessor::Mutex | Accessor::RwLock | Accessor::Write => true,
            Accessor::Option(val) => val.is_mut(),
        }
    }

    pub fn is_opt(&self) -> bool {
        match self {
            Accessor::Option(_) => true,
            Accessor::Read | Accessor::Mutex | Accessor::RwLock  |Accessor::Write => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Element {
    /// The state of the system
    State(Accessor),

    /// A reference to a component
    Component(Accessor, String),

    /// A reference to a resource
    Resource(Accessor, String),

    /// The entity ID
    Entity,

    /// A command buffer to modify the world
    CommandBuffer,

    /// A constant evaluated as rust code
    Const(String),
}

impl Element {
    pub fn init(
        &self,
        id: TokenStream,
        components: &[Component],
        resources: &[Resource],
        system: &System,
    ) -> TokenStream {
        match self {
            Element::State(accessor) => {
                let name = Ident::new(
                    &format!("sys_{}_state", system.name).to_case(Case::Snake),
                    Span::call_site(),
                );
                let state_name = system.as_ident();
                let init = accessor.wrapper_init(
                    quote::quote! {
                        self.#state_name
                    },
                    true,
                );

                quote::quote! { let #name = #init; }
            }
            Element::Resource(accessor, name) => {
                let resource = find_resource(resources, name);
                let name = Ident::new(
                    &format!("sys_{}_res_{}", system.name, name).to_case(Case::Snake),
                    Span::call_site(),
                );
                let state_name = resource.as_field_ident();
                let init = accessor.wrapper_init(
                    quote::quote! {
                        self.#state_name
                    },
                    true,
                );

                quote::quote! { let #name = #init; }
            }
            Element::Component(accessor, name) => {
                let component = find_component(components, name);
                let name = Ident::new(
                    &format!("sys_{}_comp_{}", system.name, name).to_case(Case::Snake),
                    Span::call_site(),
                );
                let field_name = component.as_ident();
                let init = accessor.wrapper_init(
                    component.storage.read_function(id, quote::quote! { self.#field_name }, accessor.is_mut(), accessor.is_opt()),
                    false,
                );

                quote::quote! {
                    let #name = #init;
                }
            }
            Element::Entity => quote::quote! { let entt = #id; },
            Element::CommandBuffer => todo!(),
            Element::Const(_) => quote::quote! {}
        }
    }

    pub fn getter(&self, system: &System) -> TokenStream {
        match self {
            Element::State(_) => {
                let name = Ident::new(
                    &format!("sys_{}_state", system.name).to_case(Case::Snake),
                    Span::call_site(),
                );

                quote::quote! {#name }
            }
            Element::Component(_, name) => {
                let name = Ident::new(
                    &format!("sys_{}_comp_{}", system.name, name).to_case(Case::Snake),
                    Span::call_site(),
                );

                quote::quote! {#name }
            }
            Element::Resource(_, name) => {
                let name = Ident::new(
                    &format!("sys_{}_res_{}", system.name, name).to_case(Case::Snake),
                    Span::call_site(),
                );

                quote::quote! {#name }
            }
            Element::Entity => quote::quote! { entt },
            Element::Const(c) => syn::parse_str(c).expect("Failed to parse const"),
            Element::CommandBuffer => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SystemKind {
    /// The system is a function that control iteration internally
    Function,

    /// The system is a function that control iteration internally and is
    /// asynchronous: this is useful when dealing with networking, file system, etc.
    AsyncFunction,

    /// The system is a function whose iteration is controlled externally
    ForEachFunction,

    /// The system is a function that control iteration internally and is
    /// asynchronous: this is useful when dealing with networking, file system, etc.
    /// This allows data parallelism meaning that all of the iterations may be waiting at
    /// the same time and there is **no guarantee** of order of execution. Its execution may
    /// even be interleaved with other systems as long as there are no write conflicts.
    ///
    /// **NOTE**: This type of system does not support writing to resources or state. To go
    /// around this limitation, you can use RwLock, Mutex or simply Atomic structure. However,
    /// with locking structure, be careful to not cause deadlocks.
    ForEachAsyncFunction,
}

impl SystemKind {
    pub fn make_run(&self, system: &System, components: &[Component], resources: &[Resource]) -> TokenStream {
        match self {
            SystemKind::ForEachFunction => {
                let function: Path =
                    syn::parse_str(&system.path).expect("Failed parsing function path");

                let mut comp_iter = quote::quote! {};
                let mut first: bool = true;
                for element in &system.signature {
                    if let Element::Component(accessor, name) = element {
                        if accessor.is_opt() {
                            continue;
                        }
                        
                        let component = find_component(components, name);
                        let bitset = component.as_bitset();
                        let new_comp = quote::quote! {
                            &self.#bitset
                        };

                        if first {
                            comp_iter = new_comp;
                        } else {
                            comp_iter =
                                quote::quote! { ::secs::hibitset::BitSetAnd(#new_comp, #comp_iter)};
                        }
                        first = false;
                    }
                }

                comp_iter = quote::quote! { ::secs::hibitset::BitSetAnd(#comp_iter, &self.alive) };

                let flag = if system.result {
                    quote::quote! { ? }
                } else {
                    quote::quote! {}
                };

                let (start_if, end_if) = if system.control_flow {
                    (quote::quote! {if !}, quote::quote! { { break; } })
                } else {
                    (quote::quote! {}, quote::quote! {;})
                };

                let inits = system
                    .signature
                    .iter()
                    .map(|elem| elem.init(quote::quote! { id }, components, resources, system));

                let refs = system.signature.iter().map(|elem| elem.getter(system));

                quote::quote! {
                    for id in #comp_iter {
                        let id = ::secs::Entity::new(id);
                        #(#inits;)*

                        #start_if #function(
                            #(#refs,)*
                        )#flag #end_if
                    }
                }
            },
            SystemKind::AsyncFunction => todo!(),
            SystemKind::Function => todo!(),
            SystemKind::ForEachAsyncFunction => todo!()
        }
    }
}
