use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether to print cargo control strings, enabled this if using from a build script.
    pub cargo_control: bool,

    /// Whether rust fmt should be ran on the output string
    pub rustfmt: bool,

    /// Component files
    pub components: Vec<PathBuf>,

    /// Resource files
    pub resources: Vec<PathBuf>,

    /// System files
    pub systems: Vec<PathBuf>,

    /// Main ECS config file
    pub main: PathBuf,
}

impl Config {
    pub fn new<P: Into<PathBuf>>(main: P) -> Self {
        Self {
            cargo_control: true,
            rustfmt: true,
            components: Vec::new(),
            resources: Vec::new(),
            systems: Vec::new(),
            main: main.into(),
        }
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

    pub fn resources<P: Into<PathBuf>>(mut self, new: P) -> Self {
        self.resources.push(new.into());
        self
    }

    pub fn add_resources<P: Into<PathBuf>>(&mut self, new: P) -> &mut Self {
        self.resources.push(new.into());
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
}
