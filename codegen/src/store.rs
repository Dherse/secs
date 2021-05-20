use proc_macro2::TokenStream;

use crate::{
    component::{Component, ComponentStorage},
    ecs::ECS,
};

pub fn make_component_store(main: &ECS, components: &[Component]) -> TokenStream {
    let component_store = main.as_component_store_ident();

    let component_types: Vec<TokenStream> =
        components.iter().map(Component::as_struct_field).collect();

    let component_bitsets: Vec<TokenStream> =
        components.iter().map(Component::as_struct_bitset).collect();

    let component_fns = components.iter().map(|comp| {
        let getters = make_getters(comp);
        let setters = make_setters(comp);

        quote::quote! {
            #getters

            #setters
        }
    });

    let comp_set = components.iter().map(|comp| {
        let name = comp.as_ident();
        let call = comp.storage.storage_init();
        quote::quote! {
            #name: #call
        }
    });

    let comp_set_with_cap = components.iter().map(|comp| {
        let name = comp.as_ident();
        let call = comp
            .storage
            .storage_init_with_capacity(quote::quote! {capacity});
        quote::quote! {
            #name: #call
        }
    });

    let comp_bitset = components.iter().map(|comp| {
        let name = comp.as_bitset();
        quote::quote! {
            #name: ::secs::hibitset::BitSet::new()
        }
    });

    let comp_bitset_with_cap = components.iter().map(|comp| {
        let name = comp.as_bitset();
        quote::quote! {
            #name: ::secs::hibitset::BitSet::with_capacity(capacity as u32)
        }
    });

    let push_calls = components.iter().map(|comp| {
        let name = comp.as_ident();
        let bitset = comp.as_bitset();
        comp.storage.clear_function(
            quote::quote! { self.#name },
            quote::quote! { self.#bitset },
            quote::quote! { entity },
        )
    });

    let delete_calls = components.iter().map(|comp| {
        let bitset = comp.as_bitset();
        let delete =
            comp.storage
                .remove_function(comp, quote::quote! { entity }, quote::quote! { exists });
        let name = comp.as_ident();

        quote::quote! {
            {
                let exists = self.#bitset.remove(entity.index());
                self.#name#delete;
            }
        }
    });

    let build_calls = components.iter().map(|comp| {
        let name = comp.as_ident();
        let bitset = comp.as_bitset();

        let delete = comp.storage.remove_function(
            comp,
            quote::quote! { builder.entity },
            quote::quote! { exists },
        );

        let set = comp.storage.write_function(
            quote::quote! { self.#name },
            quote::quote! { builder.entity },
            quote::quote! { value },
        );

        quote::quote! {
            if let Some(value) = builder.#name {
                self.#bitset.add(builder.entity.index());
                #set
            } else {
                let exists = self.#bitset.remove(builder.entity.index());
                if exists {
                    self.#name#delete;
                }
            }
        }
    });

    let name_builder = main.as_entity_builder_ident();

    quote::quote! {
        pub struct #component_store{
            max: ::std::sync::Arc<::std::sync::atomic::AtomicU32>,
            freed_rx: ::secs::crossbeam_channel::Receiver<u32>,
            freed_tx: ::secs::crossbeam_channel::Sender<u32>,
            alive: ::secs::hibitset::BitSet,
            #(#component_types,)*
            #(#component_bitsets,)*
        }

        impl #component_store {
            #[doc = "Initializes a new component store"]
            pub fn new() -> Self{
                let (tx, rx) = ::secs::crossbeam_channel::unbounded();
                Self {
                    max: ::std::sync::Arc::new(::std::sync::atomic::AtomicU32::new(0)),
                    alive: ::secs::hibitset::BitSet::new(),
                    freed_rx: rx,
                    freed_tx: tx,
                    #(#comp_set,)*
                    #(#comp_bitset,)*
                }
            }

            #[doc = "Initializes a new component store with a base capacity"]
            pub fn with_capacity(capacity: usize) -> Self{
                let (tx, rx) = ::secs::crossbeam_channel::unbounded();
                Self {
                    max: ::std::sync::Arc::new(::std::sync::atomic::AtomicU32::new(0)),
                    alive: ::secs::hibitset::BitSet::new(),
                    freed_rx: rx,
                    freed_tx: tx,
                    #(#comp_set_with_cap,)*
                    #(#comp_bitset_with_cap,)*
                }
            }

            #[doc = "Checks if an `entity` is alive"]
            pub fn alive(&self, entity: ::secs::Entity) -> bool {
                self.alive.contains(entity.index())
            }

            #[doc = "Reserves an entity id, this entity is dead until it has been built!"]
            pub fn next(&self) -> ::secs::Entity {
                if let Ok(value) = self.freed_rx.try_recv() {
                    ::secs::Entity::new(value)
                } else {
                    ::secs::Entity::new(self.max.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst))
                }
            }

            #[doc = "Adds an entity (dead or alive) to the list of alive entities and clears all of its components"]
            pub fn reset(&mut self, entity: ::secs::Entity) {
                self.alive.add(entity.index());
                #(#push_calls)*
            }

            pub fn build(&mut self, builder: #name_builder) {
                self.alive.add(builder.entity.index());
                #(#build_calls)*
            }

            #[doc = "Kills an entity, returns true if the entity was alive"]
            pub fn kill(&mut self, entity: ::secs::Entity) -> bool {
                if self.alive.remove(entity.index()) {
                    self.freed_tx.send(entity.index()).expect("Failed to queue ID reuse");
                    #(#delete_calls)*
                    true
                } else {
                    false
                }
            }

            #(#component_fns)*
        }
    }
}

fn make_setters(comp: &Component) -> TokenStream {
    let name = comp.as_ident();
    let ty = comp.as_ty();
    let add_name = comp.as_add_ident();
    let del_name = comp.as_del_ident();
    let bitset_name = comp.as_bitset();
    let set_call = comp.storage.write_function(
        quote::quote! { self.#name },
        quote::quote! { entity },
        quote::quote! { value },
    );
    let del_call =
        comp.storage
            .remove_function(comp, quote::quote! { entity }, quote::quote! { exists });
    let doc_str_add = format!(
        "Adds the component '{}' of type [`{}`] to the `entity`",
        comp.name, comp.path
    );
    let doc_str_del = format!("Removes the component '{}' of type [`{}`] from the `entity`, returns the component if it had it", comp.name, comp.path);

    quote::quote! {
        #[doc = #doc_str_add]
        pub fn #add_name(&mut self, entity: ::secs::Entity, value: #ty) -> &mut Self {
            assert!(self.alive.contains(entity.index()), "Entity is not alive");

            self.#bitset_name.add(entity.index());
            #set_call
            self
        }

        #[doc = #doc_str_del]
        pub fn #del_name(&mut self, entity: ::secs::Entity) -> Option<#ty> {
            assert!(self.alive.contains(entity.index()), "Entity is not alive");

            let exists = self.#bitset_name.remove(entity.index());
            if exists {
                self.#name#del_call
            } else {
                None
            }
        }
    }
}

fn make_getters(comp: &Component) -> TokenStream {
    let name = comp.as_ident();
    let bitset_name = comp.as_bitset();

    if let ComponentStorage::Null = comp.storage {
        let doc_str = format!(
            "Checks whether the `entity` has component '{}' of type [`{}`]",
            comp.name, comp.path
        );
        return quote::quote! {
            #[doc = #doc_str]
            pub fn #name(&self, entity: ::secs::Entity) -> bool {
                self.alive.contains(entity.index()) && self.#bitset_name.contains(entity.index())
            }
        };
    }

    let name_mut = comp.as_mut();
    let read_call = comp.storage.read_function(
        comp,
        quote::quote! { entity },
        quote::quote! { self},
        quote::quote! { self.#name},
        false,
        true,
    );
    let read_call_mut = comp.storage.read_function(
        comp,
        quote::quote! { entity },
        quote::quote! { self},
        quote::quote! { self.#name},
        true,
        true,
    );

    let ty = {
        let ty = comp.as_ty();
        quote::quote! { &#ty }
    };

    let ty_mut = {
        let ty = comp.as_ty();
        quote::quote! { &mut #ty }
    };

    let doc_str = format!(
        "Gets a reference to the component '{}' of type [`{}`] from the `entity` if it exists",
        comp.name, comp.path
    );
    let doc_str_mut = format!("Gets a mutable reference to the component '{}' of type [`{}`] from the `entity` if it exists", comp.name, comp.path);

    quote::quote! {
        #[doc = #doc_str]
        pub fn #name(&self, entity: ::secs::Entity) -> Option<#ty> {
            if !self.alive.contains(entity.index()) || !self.#bitset_name.contains(entity.index()) {
                return None;
            }

            #read_call
        }

        #[doc = #doc_str_mut]
        pub fn #name_mut(&mut self, entity: ::secs::Entity) -> Option<#ty_mut> {
            if !self.alive.contains(entity.index()) || !self.#bitset_name.contains(entity.index()) {
                return None;
            }

            #read_call_mut
        }
    }
}
