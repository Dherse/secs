#![allow(incomplete_features)]
#![feature(const_generics)]

pub mod executor;
pub mod join;
pub mod storage;

pub use crossbeam_channel;
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
