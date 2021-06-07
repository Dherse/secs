use std::path::PathBuf;

use crate::{component::Component, resource::Resource, system::System};

#[derive(Debug, Clone)]
pub struct Config<'a> {
    /// Whether to print cargo control strings, enabled this if using from a build script.
    pub cargo_control: bool,

    /// Whether rust fmt should be ran on the output string
    pub rustfmt: bool,

    /// Component files
    pub components: Vec<PathBuf>,

    /// Built components
    pub built_components: Vec<Component<'a>>,

    /// Resource files
    pub resources: Vec<PathBuf>,

    /// Built resources
    pub built_resources: Vec<Resource<'a>>,

    /// System files
    pub systems: Vec<PathBuf>,

    /// Built systems
    pub built_systems: Vec<System<'a>>,

    /// Main ECS config file
    pub main: PathBuf,
}

impl<'a> Config<'a> {
    pub fn new<P: Into<PathBuf>>(main: P) -> Self {
        Self {
            cargo_control: true,
            rustfmt: true,
            built_components: Vec::new(),
            components: Vec::new(),
            built_resources: Vec::new(),
            resources: Vec::new(),
            built_systems: Vec::new(),
            systems: Vec::new(),
            main: main.into(),
        }
    }

    pub fn rustmft(mut self, enabled: bool) -> Self {
        self.rustfmt = enabled;
        self
    }

    pub fn set_rustmft(&mut self, enabled: bool) -> &mut Self {
        self.rustfmt = enabled;
        self
    }

    pub fn cargo_control(mut self, cargo_control: bool) -> Self {
        self.cargo_control = cargo_control;
        self
    }

    pub fn set_cargo_control(&mut self, cargo_control: bool) -> &mut Self {
        self.cargo_control = cargo_control;
        self
    }

    pub fn components<P: Into<PathBuf>>(mut self, new: P) -> Self {
        self.components.push(new.into());
        self
    }

    pub fn add_components<P: Into<PathBuf>>(&mut self, new: P) -> &mut Self {
        self.components.push(new.into());
        self
    }

    pub fn component(mut self, new: Component<'a>) -> Self {
        self.built_components.push(new);
        self
    }

    pub fn add_component(&mut self, new: Component<'a>) -> &mut Self {
        self.built_components.push(new);
        self
    }

    pub fn resources<P: Into<PathBuf>>(mut self, new: P) -> Self {
        self.resources.push(new.into());
        self
    }

    pub fn add_resources<P: Into<PathBuf>>(&mut self, new: P) -> &mut Self {
        self.resources.push(new.into());
        self
    }

    pub fn resource(mut self, new: Resource<'a>) -> Self {
        self.built_resources.push(new);
        self
    }

    pub fn add_resource(&mut self, new: Resource<'a>) -> &mut Self {
        self.built_resources.push(new);
        self
    }

    pub fn systems<P: Into<PathBuf>>(mut self, new: P) -> Self {
        self.systems.push(new.into());
        self
    }

    pub fn add_systems<P: Into<PathBuf>>(&mut self, new: P) -> &mut Self {
        self.systems.push(new.into());
        self
    }

    pub fn system(mut self, new: System<'a>) -> Self {
        self.built_systems.push(new);
        self
    }

    pub fn add_system(&mut self, new: System<'a>) -> &mut Self {
        self.built_systems.push(new);
        self
    }
}
