use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use config::Config;
use proc_macro2::{Ident, Span, TokenStream};
use serde::Deserialize;
use syn::Path;

use crate::{component::Component, ecs::ECS, resource::Resource, system::System};

pub mod component;
pub mod config;
pub mod ecs;
pub mod resource;
pub mod system;

pub fn build(config: Config) -> String {
    // Load the component files
    let components: Vec<Component> = config
        .components
        .iter()
        .map(|f| load::<Vec<Component>>(&config, f))
        .collect::<Result<Vec<Vec<_>>, _>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect();

    // Load the resources file
    let resources: Vec<Resource> = config
        .resources
        .iter()
        .map(|f| load::<Vec<Resource>>(&config, f))
        .collect::<Result<Vec<Vec<_>>, _>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect();

    // Load the systems
    let systems: Vec<System> = config
        .systems
        .iter()
        .map(|f| load::<Vec<System>>(&config, f))
        .collect::<Result<Vec<Vec<_>>, _>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect();

    // Load the main ECS files
    let main: ECS = load(&config, &config.main).unwrap();

    // Regroup systems by stage (for scheduling)
    let mut systems_by_stage: HashMap<String, Vec<System>> = HashMap::new();
    for sys in &systems {
        assert!(
            main.stages.contains(&sys.stage),
            "Unknown stage: {}",
            sys.stage
        );

        if let Some(systems) = systems_by_stage.get_mut(&sys.stage) {
            systems.push(sys.clone());
        } else {
            systems_by_stage.insert(sys.stage.clone(), vec![sys.clone()]);
        }
    }

    let output_struct = make_struct(&main, &components, &resources, &systems);
    let builder = make_builder(&main, &components, &resources, &systems);

    let output = format!(
        "{}",
        quote::quote! {
            #output_struct
            #builder
        }
    );

    if config.rustfmt {
        run_rustfmt(output).unwrap()
    } else {
        output
    }
}

fn make_struct(
    main: &ECS,
    components: &[Component],
    resources: &[Resource],
    systems: &[System],
) -> TokenStream {
    let name = main.as_ident();
    let builder_name = main.as_builder_ident();

    let component_types: Vec<TokenStream> =
        components.iter().map(Component::as_struct_field).collect();

    let component_bitsets: Vec<TokenStream> =
        components.iter().map(Component::as_struct_bitset).collect();

    let resource_types: Vec<TokenStream> =
        resources.iter().map(Resource::as_struct_field).collect();

    let system_state_types: Vec<TokenStream> = systems
        .iter()
        .map(System::as_struct_field)
        .filter_map(|v| v)
        .collect();

    let derives = main
        .derive
        .iter()
        .map(|derive| Ident::new(derive, Span::call_site()));

    let err_ty: Path = syn::parse_str(
        &main
            .error
            .clone()
            .unwrap_or("Box<dyn std::error::Error>".to_owned()),
    )
    .expect("Failed to parse error type");

    let system_runs = systems
        .iter()
        .map(|sys| sys.kind.make_run(sys, components, resources));

    let component_fns = components.iter().map(|comp| {
        let name = comp.as_ident();
        let del_name = comp.as_del_ident();
        let bitset_name = comp.as_bitset();
        let ty = comp.as_ty();

        let doc_str_add = format!("Adds the component '{}' of type [`{}`] to the `entity`", comp.name, comp.path);
        let doc_str_del = format!("Removes the component '{}' of type [`{}`] from the `entity`, returns the component if it had it", comp.name, comp.path);

        let set_call = comp.storage.write_function(quote::quote! { entity }, quote::quote! { value });
        let del_call = comp.storage.remove_function(comp, quote::quote! { entity }, quote::quote! { exists });

        quote::quote! {
            #[doc = #doc_str_add]
            pub fn #name(&mut self, entity: ::secs::Entity, value: #ty) -> &mut Self {
                assert!((entity.index() as usize) < self.index_, "Entity ID is not in the existing range");
                assert!(self.alive.contains(entity.index()), "Entity is not alive");

                self.#bitset_name.add(entity.index());
                self.#name#set_call
                self
            }

            #[doc = #doc_str_del]
            pub fn #del_name(&mut self, entity: ::secs::Entity) -> Option<#ty> {
                assert!((entity.index() as usize) < self.index_, "Entity ID is not in the existing range");
                assert!(self.alive.contains(entity.index()), "Entity is not alive");

                let exists = self.#bitset_name.remove(entity.index());
                self.#name#del_call
            }
        }
    });

    let push_calls = components.iter().map(|comp| {
        let name = comp.as_ident();
        comp.storage
            .alloc_function(quote::quote! { self.#name }, quote::quote! { entity })
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

    quote::quote! {
        #[derive(#(#derives),*)]
        pub struct #name {
            index_: usize,
            alive: ::secs::hibitset::BitSet,
            #(#component_types,)*
            #(#component_bitsets,)*
            #(#resource_types,)*
            #(#system_state_types,)*
        }

        impl #name {
            #[doc = "Creates a builder for this ECS"]
            pub fn builder() -> #builder_name {
                #builder_name::new()
            }

            #[doc = "Runs the ECS"]
            pub fn run(&mut self) -> Result<(), #err_ty> {
                #(#system_runs)*

                Ok(())
            }

            #[doc = "Creates a new entity"]
            pub fn push(&mut self) -> ::secs::Entity {
                let entity = ::secs::Entity::new(self.index_ as u32);
                self.alive.add(self.index_ as u32);
                self.index_ += 1;

                #(#push_calls)*

                entity
            }

            #[doc = "Deletes an entity, returns true if this entity was alive"]
            pub fn delete(&mut self, entity: ::secs::Entity) -> bool {
                if self.alive.remove(entity.index()) {
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

fn make_builder(
    main: &ECS,
    components: &[Component],
    resources: &[Resource],
    systems: &[System],
) -> TokenStream {
    let ecs_name = main.as_ident();
    let name = main.as_builder_ident();

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

    let resource_types: Vec<TokenStream> =
        resources.iter().map(Resource::as_builder_field).collect();

    let system_state_types: Vec<TokenStream> = systems
        .iter()
        .map(System::as_builder_field)
        .filter_map(|v| v)
        .collect();

    let res_functions: Vec<TokenStream> = resources
        .iter()
        .map(|res| {
            let ty = res.as_ty();
            let name = res.as_field_ident();

            let doc_str = format!("Sets the resource '{}' of type [`{}`]", res.name, res.path);

            quote::quote! {
                #[doc = #doc_str]
                pub fn #name(mut self, value: #ty) -> Self {
                    self.#name = value;
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

    quote::quote! {
        #[derive(Default)]
        pub struct #name {
            #(#resource_types,)*
            #(#system_state_types,)*
        }

        impl #name {
            #[doc = "Creates a new builder"]
            pub fn new() -> Self {
                Self::default()
            }

            #[doc = "Builds the builder into the ECS"]
            pub fn build(self) -> #ecs_name {
                #ecs_name {
                    alive: ::secs::hibitset::BitSet::new(),
                    index_: 0,
                    #(#res_set,)*
                    #(#state_set,)*
                    #(#comp_set,)*
                    #(#comp_bitset,)*
                }
            }

            #[doc = "Builds the builder into the ECS with a capacity"]
            pub fn with_capacity(self, capacity: usize) -> #ecs_name {
                #ecs_name {
                    alive: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
                    index_: 0,
                    #(#res_set,)*
                    #(#state_set,)*
                    #(#comp_set_with_cap,)*
                    #(#comp_bitset_with_cap,)*
                }
            }

            #(#res_functions)*

            #(#state_functions)*
        }
    }
}

fn load<O: for<'de> Deserialize<'de>>(config: &Config, path: &PathBuf) -> Result<O, io::Error> {
    if config.cargo_control {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let mut file = File::open(path)?;

    let mut buf = String::with_capacity(4096);
    file.read_to_string(&mut buf)?;

    match ron::from_str(&buf) {
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        Ok(o) => Ok(o),
    }
}

fn run_rustfmt(source: String) -> Result<String, io::Error> {
    // This is code shamefully yoinked from the bindgen repo, all credits to them

    let rustfmt = "rustfmt"; // TODO: search for the executable;
    let mut cmd = Command::new(&*rustfmt);

    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());

    let mut child = cmd.spawn()?;
    let mut child_stdin = child.stdin.take().unwrap();
    let mut child_stdout = child.stdout.take().unwrap();

    let source = source.to_owned();

    // Write to stdin in a new thread, so that we can read from stdout on this
    // thread. This keeps the child from blocking on writing to its stdout which
    // might block us from writing to its stdin.
    let stdin_handle = ::std::thread::spawn(move || {
        let _ = child_stdin.write_all(source.as_bytes());
        source
    });

    let mut output = vec![];
    io::copy(&mut child_stdout, &mut output)?;

    let status = child.wait()?;
    let source = stdin_handle.join().expect(
        "The thread writing to rustfmt's stdin doesn't do \
            anything that could panic",
    );

    match String::from_utf8(output) {
        Ok(bindings) => match status.code() {
            Some(0) => Ok(bindings),
            Some(2) => Err(io::Error::new(
                io::ErrorKind::Other,
                "Rustfmt parsing errors.".to_string(),
            )),
            Some(3) => {
                println!("cargo:warning=Rustfmt could not format some lines.");
                Ok(bindings)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "Internal rustfmt error".to_string(),
            )),
        },
        _ => Ok(source),
    }
}

pub fn find_component<'a>(components: &'a [Component], name: &String) -> &'a Component {
    for comp in components {
        if &comp.name == name {
            return comp;
        }
    }

    panic!("Unknown component: {}", name);
}

pub fn find_resource<'a>(resources: &'a [Resource], name: &String) -> &'a Resource {
    for res in resources {
        if &res.name == name {
            return res;
        }
    }

    panic!("Unknown resource: {}", name);
}
