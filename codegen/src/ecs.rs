use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ECS {
    /// The path of the output data structure
    pub name: String,

    /// The error type, if none, default to `Box<dyn Error>`
    pub error: Option<String>,

    /// List of stages in this ECS, a stage is a group
    /// of system followed by a barrier and a flush.
    /// The barrier means that all systems must be done executing
    /// before reaching the barrier.
    /// The flush means that all command buffers will be flushed
    /// at that point.
    pub stages: Vec<String>,
}

impl ECS {
    pub fn as_ident(&self) -> Ident {
        Ident::new(&self.name.to_case(Case::UpperCamel), Span::call_site())
    }

    pub fn as_builder_ident(&self) -> Ident {
        Ident::new(
            &format!("{}Builder", self.name).to_case(Case::UpperCamel),
            Span::call_site(),
        )
    }

    pub fn as_component_store_ident(&self) -> Ident {
        Ident::new(
            &format!("{}ComponentStore", self.name).to_case(Case::UpperCamel),
            Span::call_site(),
        )
    }

    pub fn as_entity_builder_ident(&self) -> Ident {
        Ident::new(
            &format!("{}EntityBuilder", self.name).to_case(Case::UpperCamel),
            Span::call_site(),
        )
    }
}
