pub mod executor;

pub use fxhash;
pub use hibitset;
pub use parking_lot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity(u32);

impl Entity {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn index(&self) -> u32 {
        self.0
    }
}
