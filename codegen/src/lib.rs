use std::{
    fs::File,
    io::{self, Read, Write},
    process::{Command, Stdio},
};

use config::Config;
use fxhash::{FxHashMap, FxHashSet};
use proc_macro2::{Span, TokenStream};
use syn::Ident;

use crate::{
    builder::make_builder, command::build_command_buffer, component::Component, ecs::ECS,
    entity::make_entity_builder, resource::Resource, store::make_component_store, system::System,
};

mod builder;
mod command;
mod component;
pub mod config;
mod ecs;
mod entity;
mod resource;
mod store;
mod system;

pub fn build<'a>(config: Config<'a>) -> String {
    // Load the component files
    let comp_contents = config.components.iter()
        .map(|f| {
            let mut out = String::with_capacity(4096);
            let mut file = File::open(f)?;
            file.read_to_string(&mut out)?;

            Ok(out)
        }).collect::<Result<Vec<String>, io::Error>>().unwrap();

    let mut components = Vec::new();
    components.extend(config.built_components);
    comp_contents.iter().for_each(|c| {
        components.extend(ron::from_str::<Vec<Component>>(c).unwrap());
    });

    // Load the component files
    let res_contents = config.resources.iter()
        .map(|f| {
            let mut out = String::with_capacity(4096);
            let mut file = File::open(f)?;
            file.read_to_string(&mut out)?;

            Ok(out)
        }).collect::<Result<Vec<String>, io::Error>>().unwrap();

    let mut resources = Vec::new();
    resources.extend(config.built_resources);
    res_contents.iter().for_each(|c| {
        resources.extend(ron::from_str::<Vec<Resource>>(c).unwrap());
    });

    // Load the component files
    let sys_contents = config.systems.iter()
        .map(|f| {
            let mut out = String::with_capacity(4096);
            let mut file = File::open(f)?;
            file.read_to_string(&mut out)?;

            Ok(out)
        }).collect::<Result<Vec<String>, io::Error>>().unwrap();

    let mut systems = Vec::new();
    systems.extend(config.built_systems);
    sys_contents.iter().for_each(|c| {
        systems.extend(ron::from_str::<Vec<System>>(c).unwrap());
    });

    let main_contents = {
        let mut out = String::with_capacity(4096);
        let mut file = File::open(&config.main).unwrap();
        file.read_to_string(&mut out).unwrap();

        out
    };

    let main: ECS = match ron::from_str(&main_contents) {
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        Ok(o) => Ok(o),
    }.unwrap();

    let component_lifetimes = components
        .iter()
        .filter_map(|comp| comp.lifetimes.clone())
        .flatten()
        .collect::<FxHashSet<_>>();

    let system_lifetimes = systems
        .iter()
        .filter_map(|sys| sys.lifetimes.clone())
        .flatten()
        .collect::<FxHashSet<_>>();

    let resource_lifetimes = resources
        .iter()
        .filter_map(|res| res.lifetimes.clone())
        .flatten()
        .collect::<FxHashSet<_>>();

    // Regroup all lifetimes
    let mut lifetimes = FxHashSet::default();
    lifetimes.extend(component_lifetimes.iter().cloned());
    lifetimes.extend(system_lifetimes.iter().cloned());
    lifetimes.extend(resource_lifetimes.iter().cloned());

    let generics = make_generics(
        lifetimes,
        component_lifetimes,
        system_lifetimes,
        resource_lifetimes,
    );

    let output_struct = make_struct(&main, &components, &resources, &systems, &generics);
    let builder = make_builder(&main, &resources, &systems, &generics);
    let component_store = make_component_store(&main, &components, &generics);
    let entity_builder = make_entity_builder(&main, &components, &generics);
    let command_buffer = build_command_buffer(&main, &components, &generics);

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

fn make_struct<'a>(
    main: &ECS<'a>,
    components: &[Component<'a>],
    resources: &[Resource<'a>],
    systems: &[System<'a>],
    generics: &GenericOutput,
) -> TokenStream {
    let name = main.as_ident();
    let builder_name = main.as_builder_ident();

    let resource_types: Vec<TokenStream> =
        resources.iter().map(Resource::as_struct_field).collect();

    let system_state_types: Vec<TokenStream> = systems
        .iter()
        .map(System::as_struct_field)
        .flatten()
        .collect();

    let err_ty: TokenStream = syn::parse_str(
        &main
            .error
            .clone()
            .unwrap_or_else(|| "Box<dyn std::error::Error>"),
    )
    .expect("Failed to parse error type");

    // Regroup systems by stage (for scheduling)
    let mut systems_by_stage: FxHashMap<&'a str, Vec<System<'a>>> = FxHashMap::default();
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

    let mut res_fns = Vec::new();
    for res in resources {
        let name = res.as_field_ident();
        let ty = res.as_ty();

        let get_mut = Ident::new(&format!("{}_mut", res.as_field_name()), Span::call_site());
        let set = Ident::new(&format!("set_{}", res.as_field_name()), Span::call_site());

        let get_doc = format!(
            "Gets a reference to the resource '{}' of type [`{}`]",
            res.name, res.path
        );
        let get_mut_doc = format!(
            "Gets a mutable reference to the resource '{}' of type [`{}`]",
            res.name, res.path
        );
        let set_doc = format!("Sets the resource '{}' of type [`{}`]", res.name, res.path);

        res_fns.push(quote::quote! {
            #[doc = #get_doc]
            pub fn #name(&self) -> &#ty {
                &self.#name
            }

            #[doc = #get_mut_doc]
            pub fn #get_mut(&mut self) -> &mut #ty {
                &mut self.#name
            }

            #[doc = #set_doc]
            pub fn #set(&mut self, mut value: #ty) -> #ty {
                ::std::mem::swap(&mut value, &mut self.#name);
                value
            }
        });
    }

    let component_store = main.as_component_store_ident();
    let entity_builder = main.as_entity_builder_ident();
    let command_buffer = main.as_command_buffer_ident();

    let ecs_generics = &generics.ecs;
    let builder_generics = &generics.builder;
    let component_generics = &generics.components;

    quote::quote! {
        pub struct #name#ecs_generics {
            components: #component_store#component_generics,
            command_buffer: #command_buffer#component_generics,
            #(#resource_types,)*
            #(#system_state_types,)*
        }

        impl#ecs_generics #name#ecs_generics {
            #[doc = "Creates a builder for this ECS"]
            pub fn builder() -> #builder_name#builder_generics {
                #builder_name::new()
            }

            #[doc = "Runs the ECS"]
            pub fn run(&mut self) -> Result<(), #err_ty> {
                let components = &mut self.components;

                #(#system_runs)*

                Ok(())
            }

            #[doc = "Returns a new entity builder"]
            pub fn next(&self) -> #entity_builder#component_generics {
                <#entity_builder>::new(self.components.next())
            }

            #[doc = "Takes the `builder` and creates an entity in the ECS"]
            pub fn build(&mut self, builder: #entity_builder#component_generics) {
                self.components.build(builder);
            }

            #[doc = "Gets an immutable reference to the component store"]
            pub fn components(&self) -> &#component_store#component_generics {
                &self.components
            }

            #[doc = "Gets a mutable reference to the component store"]
            pub fn components_mut(&mut self) -> &mut #component_store#component_generics {
                &mut self.components
            }

            #(#res_fns)*
        }
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

pub fn find_component<'a, 'b: 'a>(components: &'a [Component<'b>], name: &str) -> &'a Component<'b> {
    for comp in components {
        if comp.name == name {
            return comp;
        }
    }

    panic!("Unknown component: {}", name);
}

pub fn find_resource<'a, 'b: 'a>(resources: &'a [Resource<'b>], name: &str) -> &'a Resource<'b> {
    for res in resources {
        if res.name == name {
            return res;
        }
    }

    panic!("Unknown resource: {}", name);
}

pub(crate) struct GenericOutput {
    pub ecs: TokenStream,
    pub builder: TokenStream,
    pub components: TokenStream,
}

fn make_generics<'a>(
    lifetimes: FxHashSet<&'a str>,
    component_lifetimes: FxHashSet<&'a str>,
    system_lifetimes: FxHashSet<&'a str>,
    resource_lifetimes: FxHashSet<&'a str>,
) -> GenericOutput {
    if lifetimes.is_empty() {
        return GenericOutput {
            ecs: quote::quote! {},
            builder: quote::quote! {},
            components: quote::quote! {},
        };
    }

    let ecs: TokenStream = syn::parse_str(
        &lifetimes
            .iter()
            .map(|l| format!("'{}", l))
            .collect::<Vec<String>>()
            .join(", "),
    )
    .expect("Failed to build lifetime list");

    let builder_lifetimes = system_lifetimes
        .iter()
        .chain(resource_lifetimes.iter())
        .map(|l| format!("'{}", l))
        .collect::<Vec<String>>();

    let builder: TokenStream = if builder_lifetimes.is_empty() {
        quote::quote! {}
    } else {
        let builder: TokenStream =
            syn::parse_str(&builder_lifetimes.join(", ")).expect("Failed to build lifetime list");
        quote::quote! { <#builder> }
    };

    let component_lifetimes = component_lifetimes
        .iter()
        .map(|l| format!("'{}", l))
        .collect::<Vec<String>>();

    let components: TokenStream = if component_lifetimes.is_empty() {
        quote::quote! {}
    } else {
        let components: TokenStream =
            syn::parse_str(&component_lifetimes.join(", ")).expect("Failed to build lifetime list");
        quote::quote! { <#components> }
    };

    GenericOutput {
        ecs: quote::quote! { <#ecs> },
        builder,
        components,
    }
}
