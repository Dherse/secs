use proc_macro2::TokenStream;

use crate::{component::Component, ecs::ECS};

pub fn build_command_buffer(main: &ECS, components: &[Component]) -> TokenStream {
    let name = main.as_command_buffer_ident();
    let entity_builder = main.as_entity_builder_ident();
    let component_store = main.as_component_store_ident();

    let component_edit = components.iter().map(|comp| {
        let name_add = comp.as_add_ident();
        let name_del = comp.as_del_ident();
        let ty = comp.as_ty();

        quote::quote! {
            #name_add: ::secs::fxhash::FxHashMap<::secs::Entity, #ty>,
            #name_del: ::secs::fxhash::FxHashSet<::secs::Entity>,
        }
    });

    let component_init = components.iter().map(|comp| {
        let name_add = comp.as_add_ident();
        let name_del = comp.as_del_ident();

        quote::quote! {
            #name_add: ::secs::fxhash::FxHashMap::default(),
            #name_del: ::secs::fxhash::FxHashSet::default(),
        }
    });

    let component_fns = components.iter().map(|comp| {
        let name = comp.as_ident();
        let name_add = comp.as_add_ident();
        let name_del = comp.as_del_ident();
        let ty = comp.as_ty();

        let doc_add = format!("Schedule the addition of the component '{}' of type [`{}`] to the `entity`", comp.name, comp.path);
        let doc_del = format!("Schedule the removal of the component '{}' of type [`{}`] to the `entity`", comp.name, comp.path);
        
        quote::quote! {
            #[doc = #doc_add]
            pub fn #name(&mut self, entity: ::secs::Entity, value: #ty) -> &mut Self {
                self.#name_add.insert(entity, value);
                self
            }

            #[doc = #doc_del]
            pub fn #name_del(&mut self, entity: ::secs::Entity) -> &mut Self {
                self.#name_del.insert(entity);
                self
            }
        }
    });

    let component_apply = components.iter().map(|comp| {
        let name_add = comp.as_add_ident();
        let name_del = comp.as_del_ident();
        
        quote::quote! {
            for (entity, value) in self.#name_add.drain() {
                if store.alive(entity) {
                    store.#name_add(entity, value);
                }
            }

            for entity in self.#name_del.drain() {
                if store.alive(entity) {
                    store.#name_del(entity);
                }
            }
        }
    });

    quote::quote! {
        pub struct #name {
            next: ::std::sync::Arc<::std::sync::atomic::AtomicU32>,
            receiver: ::secs::crossbeam_channel::Receiver<u32>,
            new_entities: Vec<#entity_builder>,
            deleted_entities: ::secs::fxhash::FxHashSet<::secs::Entity>,
            #(#component_edit)*
        }

        impl #name {
            #[doc = "Creates a new command buffer"]
            fn new(store: &#component_store) -> Self {
                Self {
                    new_entities: Vec::new(),
                    next: ::std::sync::Arc::clone(&store.max),
                    receiver: store.freed_rx.clone(),
                    deleted_entities: ::secs::fxhash::FxHashSet::default(),
                    #(#component_init)*
                }
            }

            #[doc = "Schedules the creation of an entity, already reserving its ID"]
            pub fn entity<F: Fn(::secs::Entity, &mut #entity_builder)>(&mut self, fun: F) -> ::secs::Entity {
                let entity = if let Ok(value) = self.receiver.try_recv() {
                    ::secs::Entity::new(value)
                } else {
                    ::secs::Entity::new(self.next.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst))
                };

                let mut entity_builder = #entity_builder::new(entity);
                fun(entity, &mut entity_builder);
                self.new_entities.push(entity_builder);

                entity
            }

            #[doc = "Applied the command buffer to the component store clearing the buffer afterwards"]
            pub fn build(&mut self, store: &mut #component_store) {
                // First we do the deletion to clean everything up
                self.deleted_entities.drain().for_each(|entity| { store.kill(entity); });

                // Then we build the new entities
                self.new_entities.drain(..).for_each(|builder| store.build(builder));

                // Then we apply component modifications
                #(#component_apply)*
            }

            #[doc = "Schedules the deletion of an entity"]
            pub fn delete(&mut self, entity: ::secs::Entity) -> &mut Self {
                self.deleted_entities.insert(entity);
                self
            }

            #(#component_fns)*
        }
    }
}