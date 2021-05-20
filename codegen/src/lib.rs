use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use config::Config;
use proc_macro2::TokenStream;
use serde::Deserialize;

use crate::{command::build_command_buffer, component::Component, ecs::ECS, entity::make_entity_builder, resource::Resource, store::make_component_store, system::System};

mod command;
mod component;
pub mod config;
mod ecs;
mod entity;
mod resource;
mod store;
mod system;

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

    let output_struct = make_struct(&main, &components, &resources, &systems);
    let builder = make_builder(&main, &resources, &systems);
    let component_store = make_component_store(&main, &components);
    let entity_builder = make_entity_builder(&main, &components);
    let command_buffer = build_command_buffer(&main, &components);

    let output = format!(
        "{}",
        quote::quote! {
            #![allow(unused_variables, dead_code)]
            #output_struct
            #builder
            #component_store
            #entity_builder
            #command_buffer
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

    let resource_types: Vec<TokenStream> =
        resources.iter().map(Resource::as_struct_field).collect();

    let system_state_types: Vec<TokenStream> = systems
        .iter()
        .map(System::as_struct_field)
        .filter_map(|v| v)
        .collect();

    let err_ty: TokenStream = syn::parse_str(
        &main
            .error
            .clone()
            .unwrap_or("Box<dyn std::error::Error>".to_owned()),
    )
    .expect("Failed to parse error type");

    // Regroup systems by stage (for scheduling)
    let mut systems_by_stage: HashMap<String, Vec<System>> = HashMap::new();
    for sys in systems {
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

    let mut system_runs = Vec::new();
    for stage in &main.stages {
        if let Some(systems) = systems_by_stage.get(stage) {
            for system in systems {
                system_runs.push(system.kind.make_run(main, system, components, resources));
            }

            system_runs.push(quote::quote! {
                self.command_buffer.build(&mut self.components);
            });
        }
    }

    let component_store = main.as_component_store_ident();
    let entity_builder = main.as_entity_builder_ident();
    let command_buffer = main.as_command_buffer_ident();

    quote::quote! {
        pub struct #name {
            components: #component_store,
            command_buffer: #command_buffer,
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
                let components = &mut self.components;

                #(#system_runs)*

                Ok(())
            }

            #[doc = "Returns a new entity builder"]
            pub fn next(&self) -> #entity_builder {
                #entity_builder::new(self.components.next())
            }

            pub fn build(&mut self, builder: #entity_builder) {
                self.components.build(builder);
            }

            #[doc = "Gets an immutable reference to the component store"]
            pub fn components(&self) -> &#component_store {
                &self.components
            }

            #[doc = "Gets a mutable reference to the component store"]
            pub fn components_mut(&mut self) -> &mut #component_store {
                &mut self.components
            }
        }
    }
}

fn make_builder(main: &ECS, resources: &[Resource], systems: &[System]) -> TokenStream {
    let ecs_name = main.as_ident();
    let name = main.as_builder_ident();
    let store = main.as_component_store_ident();
    let command_buffer = main.as_command_buffer_ident();

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
                let components = #store::new();
                #ecs_name {
                    command_buffer: #command_buffer::new(&components),
                    components,
                    #(#res_set,)*
                    #(#state_set,)*
                }
            }

            #[doc = "Builds the builder into the ECS with a capacity"]
            pub fn with_capacity(self, capacity: usize) -> #ecs_name {
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
