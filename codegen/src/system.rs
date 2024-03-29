use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use serde::{Deserialize, Serialize};

use crate::{component::Component, ecs::ECS, find_component, find_resource, resource::Resource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System<'a> {
    /// The name of the system
    pub name: &'a str,

    /// The path of the function
    pub path: &'a str,

    /// The type of the system
    pub kind: SystemKind,

    /// Allows the function to stop the iteration if needed. Only available for [`SystemKind::ForEachFunction`].
    pub control_flow: bool,

    /// Whether this function returns a result, note that the result **must** implements [`Into`] for the ECS' error type.
    pub result: bool,

    /// The stage in which to execute this system
    pub stage: &'a str,

    /// The state of this system, these must be specified when the system starts or implement [`Default`]
    pub state: Option<&'a str>,

    /// List of lifetimes the `state` contains
    pub lifetimes: Option<Vec<&'a str>>,

    // Signature of the system
    pub signature: Vec<Element>,
}

impl<'a> System<'a> {
    pub fn as_field_name(&self) -> String {
        format!("system_{}", self.name.to_case(Case::Snake))
    }

    pub fn as_ident(&self) -> Ident {
        Ident::new(&self.as_field_name(), Span::call_site())
    }

    pub fn as_ty(&self) -> Option<TokenStream> {
        Some(syn::parse_str(&self.state.as_ref()?).expect("Failed to parse path"))
    }

    pub fn as_struct_field(&self) -> Option<TokenStream> {
        let name = self.as_ident();
        let ty: TokenStream = self.as_ty()?;

        Some(quote::quote! {
            #name: #ty
        })
    }

    pub fn as_builder_field(&self) -> Option<TokenStream> {
        let name = self.as_ident();
        let ty: TokenStream = self.as_ty()?;

        Some(quote::quote! {
            #name: Option<#ty>
        })
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
            Accessor::Option(val) => val.wrapper_init(content, bare),
        }
    }

    pub fn storage(
        &self,
        content: TokenStream,
        kind: TokenStream,
        bitset: TokenStream,
    ) -> TokenStream {
        match self {
            Accessor::Read => {
                quote::quote! {
                    ::secs::storage::Read::new(
                        ::secs::storage::ReadStorage::#kind(#content),
                        #bitset,
                    )
                }
            }
            Accessor::Write => {
                quote::quote! {
                    ::secs::storage::Write::new(
                        ::secs::storage::WriteStorage::#kind(#content),
                        #bitset,
                    )
                }
            }
            Accessor::Mutex => panic!("Cannot use mutex in non for-each systems"),
            Accessor::RwLock => panic!("Cannot use read-write locks in non for-each systems"),
            Accessor::Option(_) => {
                panic!("Cannot use optional accessors in non for-each systems")
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
            Accessor::Read | Accessor::Mutex | Accessor::RwLock | Accessor::Write => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Element {
    /// The state of the system
    State(Accessor),

    /// A reference to a component
    Component(Accessor, String),

    /// A required or not component that does not need to be queried
    Filter(bool, String),

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
        this: TokenStream,
        id: TokenStream,
        components: &[Component],
        resources: &[Resource],
        system: &System,
        is_async: bool,
    ) -> TokenStream {
        match self {
            Element::State(accessor) => {
                // Bypass if we're in async
                if is_async {
                    return quote::quote! {};
                }

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
                // Bypass if we're in async
                if is_async && accessor.is_mut() {
                    return quote::quote! {};
                }

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
                    component.storage.read_function(
                        component,
                        id,
                        quote::quote! { #this },
                        quote::quote! { #this.#field_name },
                        accessor.is_mut(),
                        accessor.is_opt(),
                    ),
                    false,
                );

                quote::quote! {
                    let #name = #init;
                }
            }
            Element::Entity => quote::quote! { let entt = #id; },
            Element::CommandBuffer => quote::quote! {},
            Element::Const(_) | Element::Filter(_, _) => quote::quote! {},
        }
    }

    pub fn storage(
        &self,
        system: &System,
        this: TokenStream,
        components: &[Component],
        resources: &[Resource],
    ) -> TokenStream {
        match self {
            Element::State(accessor) => {
                let state_name = system.as_ident();
                accessor.wrapper_init(
                    quote::quote! {
                        self.#state_name
                    },
                    true,
                )
            }
            Element::Component(accessor, name) => {
                let component = find_component(components, name);
                let field = component.as_ident();

                let reference = if accessor.is_mut() {
                    quote::quote! { &mut }
                } else {
                    quote::quote! { & }
                };
                let content = quote::quote! { #reference#this.#field };

                let kind = component.storage.as_kind();

                let bitset = component.as_bitset();
                let bitset = quote::quote! { &#this.#bitset};

                let init = accessor.storage(content, kind, bitset);

                quote::quote! {
                    #init
                }
            }
            Element::Entity => {
                quote::quote! {
                    ::secs::storage::Entities::new(&#this.alive)
                }
            }
            Element::Resource(accessor, name) => {
                let resource = find_resource(resources, name);
                let resource_name = resource.as_field_ident();
                let init = accessor.wrapper_init(
                    quote::quote! {
                        self.#resource_name
                    },
                    true,
                );

                quote::quote! {
                    #init
                }
            }
            Element::CommandBuffer => quote::quote! { &mut #this.command_buffer, },
            Element::Const(c) => {
                let expr: TokenStream = syn::parse_str(c).expect("Failed to parse const");

                quote::quote! {
                    #expr
                }
            }
            Element::Filter(_, _) => panic!("Filter are not support in non for-each systems"),
        }
    }

    pub fn getter(&self, system: &System, this: TokenStream) -> TokenStream {
        match self {
            Element::State(_) => {
                let name = Ident::new(
                    &format!("sys_{}_state", system.name).to_case(Case::Snake),
                    Span::call_site(),
                );

                quote::quote! { #name, }
            }
            Element::Component(_, name) => {
                let name = Ident::new(
                    &format!("sys_{}_comp_{}", system.name, name).to_case(Case::Snake),
                    Span::call_site(),
                );

                quote::quote! { #name, }
            }
            Element::Resource(_, name) => {
                let name = Ident::new(
                    &format!("sys_{}_res_{}", system.name, name).to_case(Case::Snake),
                    Span::call_site(),
                );

                quote::quote! { #name, }
            }
            Element::Entity => quote::quote! { entt, },
            Element::Const(c) => {
                let expr: TokenStream = syn::parse_str(c).expect("Failed to parse const");

                quote::quote! {
                    #expr,
                }
            }
            Element::CommandBuffer => {
                quote::quote! { &mut #this.command_buffer, }
            }
            Element::Filter(_, _) => quote::quote! {},
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
    pub fn make_run(
        &self,
        main: &ECS,
        system: &System,
        components: &[Component],
        resources: &[Resource],
    ) -> TokenStream {
        let function: TokenStream =
            syn::parse_str(&system.path).expect("Failed parsing function path");

        // Ensuring there are no references twice
        {
            let mut components = HashSet::<String>::new();
            let mut resources = HashSet::<String>::new();
            let mut state = false;
            let mut command_buffer = false;
            for element in &system.signature {
                match element {
                    Element::State(_) => {
                        if state {
                            panic!("System {} asks for state twice", system.name);
                        } else {
                            state = true;
                        }
                    }
                    Element::Component(_, name) => {
                        if components.contains(name) {
                            panic!(
                                "System {} asks for component {} more than once",
                                system.name, name
                            );
                        } else {
                            components.insert(name.clone());
                        }
                    }
                    Element::Resource(_, name) => {
                        if resources.contains(name) {
                            panic!(
                                "System {} asks for resource {} more than once",
                                system.name, name
                            );
                        } else {
                            resources.insert(name.clone());
                        }
                    }
                    Element::CommandBuffer => {
                        if command_buffer {
                            panic!(
                                "System {} asks for move than one command buffer",
                                system.name
                            );
                        } else {
                            command_buffer = true;
                        }
                    }
                    Element::Entity | Element::Const(_) | Element::Filter(_, _) => {}
                }
            }
        }

        let mut comp_iter = quote::quote! {};
        let mut first: bool = true;
        for element in &system.signature {
            match element {
                Element::Component(accessor, name) => {
                    if accessor.is_opt() {
                        continue;
                    }

                    let component = find_component(components, name);
                    let bitset = component.as_bitset();
                    let new_comp = quote::quote! {
                        &components.#bitset
                    };

                    if first {
                        comp_iter = new_comp;
                    } else {
                        comp_iter =
                            quote::quote! { ::secs::hibitset::BitSetAnd(#new_comp, #comp_iter)};
                    }
                    first = false;
                }
                Element::Filter(not, name) => {
                    let component = find_component(components, name);
                    let bitset = component.as_bitset();
                    let new_comp = if *not {
                        quote::quote! {
                            ::secs::hibitset::BitSetNot(&components.#bitset)
                        }
                    } else {
                        quote::quote! {
                            &components.#bitset
                        }
                    };

                    if first {
                        comp_iter = new_comp;
                    } else {
                        comp_iter =
                            quote::quote! { ::secs::hibitset::BitSetAnd(#new_comp, #comp_iter)};
                    }
                    first = false;
                }
                _ => continue,
            }
        }

        // comp_iter = quote::quote! { ::secs::hibitset::BitSetAnd(#comp_iter, &components.alive) };

        match self {
            SystemKind::ForEachFunction => {
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

                let inits = system.signature.iter().map(|elem| {
                    elem.init(
                        quote::quote! { components },
                        quote::quote! { id },
                        components,
                        resources,
                        system,
                        false,
                    )
                });

                let refs = system
                    .signature
                    .iter()
                    .map(|elem| elem.getter(system, quote::quote! { self }));

                quote::quote! {
                    for id in #comp_iter {
                        let id = ::secs::Entity::new(id);
                        #(#inits;)*

                        #start_if #function(
                            #(#refs)*
                        )#flag #end_if
                    }
                }
            }
            SystemKind::Function => {
                let flag = if system.result {
                    quote::quote! { ? }
                } else {
                    quote::quote! {}
                };

                let storages = system.signature.iter().map(|elem| {
                    elem.storage(system, quote::quote! { components }, components, resources)
                });

                quote::quote! {
                    {
                        #function(
                            #(#storages),*
                        )#flag
                    }
                }
            }
            SystemKind::ForEachAsyncFunction => {
                let store = main.as_component_store_ident();
                let futures = Ident::new(
                    &format!("futures_{}", system.name).to_case(Case::ScreamingSnake),
                    Span::call_site(),
                );
                let function: TokenStream =
                    syn::parse_str(&system.path).expect("Failed to parse function path");

                let inits = system.signature.iter().map(|elem| {
                    elem.init(
                        quote::quote! { this },
                        quote::quote! { id },
                        components,
                        resources,
                        system,
                        true,
                    )
                });

                let refs = system
                    .signature
                    .iter()
                    .map(|elem| elem.getter(system, quote::quote! { this }));

                quote::quote! {
                    thread_local! {
                        static #futures: ::std::cell::RefCell<*mut ()> = ::std::cell::RefCell::new(::std::ptr::null_mut());
                    }

                    #futures.with(|f| {
                        use secs::hibitset::BitSetLike;

                        let mut futures = unsafe {
                            if f.borrow().is_null() {
                                let value = Box::leak(Box::new(Vec::new()));
                                *f.borrow_mut() = value as *mut _ as *mut ();
                                value
                            } else {
                                &mut *(*f.borrow() as *mut Vec<_>)
                            }
                        };

                        let this = components as *mut #store;

                        let iter = #comp_iter.iter().map(|id| {
                            let id = ::secs::Entity::new(id);
                            let this = unsafe { &mut *this };
                            #(#inits;)*

                            #function(
                                #(#refs,)*
                            )
                        });

                        ::secs::executor::run_all(iter, &mut futures);

                        futures.clear();
                    });
                }
            }
            SystemKind::AsyncFunction => todo!(),
        }
    }
}
