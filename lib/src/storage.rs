use fxhash::FxHashMap;
use hibitset::{BitSet, BitSetLike, BitSetNot};
use std::{collections::BTreeMap, ops::Not};

use crate::Entity;

pub trait Storage: SimpleStorage {
    /// Type of the optional storage
    type OptStorage: SimpleStorage;

    /// Type of the not storage
    type NotStorage: SimpleStorage;

    /// Returns the storage as an optional storage
    fn opt(self) -> Self::OptStorage;

    /// Returns the storage as an exclude storage
    fn not(self) -> Self::NotStorage;
}

pub trait SimpleStorage {
    /// Name of the contents
    const NAME: &'static str;

    /// Contents of this storage
    type Element;

    /// Type of the mask
    type Mask: BitSetLike;

    /// Gets the mask
    fn mask(&self) -> Self::Mask;

    /// Gets an element from the storage
    unsafe fn get(&mut self, entity: u32) -> Self::Element;
}

pub enum ReadStorage<'sys, T> {
    Vec(&'sys Vec<Option<T>>),
    HashMap(&'sys FxHashMap<Entity, T>),
    BTreeMap(&'sys BTreeMap<Entity, T>),
}

impl<'sys, T: 'sys> Copy for ReadStorage<'sys, T> {}

impl<'sys, T: 'sys> Clone for ReadStorage<'sys, T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone, Copy)]
pub struct Read<'sys, T: 'sys, const NAME: &'static str> {
    storage: ReadStorage<'sys, T>,
    bitset: &'sys BitSet,
}

impl<'sys, T: 'sys, const NAME: &'static str> Read<'sys, T, NAME> {
    pub fn new(storage: ReadStorage<'sys, T>, bitset: &'sys BitSet) -> Self {
        Self { storage, bitset }
    }
}

impl<'sys, T: 'sys, const NAME: &'static str> SimpleStorage for Read<'sys, T, NAME> {
    const NAME: &'static str = NAME;

    type Element = &'sys T;

    /// Type of the mask
    type Mask = &'sys BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        &self.bitset
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        match self.storage {
            ReadStorage::Vec(vec) => vec.get(entity as usize).unwrap().as_ref().unwrap(),
            ReadStorage::HashMap(map) => map.get(&Entity(entity)).unwrap(),
            ReadStorage::BTreeMap(map) => map.get(&Entity(entity)).unwrap(),
        }
    }
}

impl<'sys, T: 'sys, const NAME: &'static str> Storage for Read<'sys, T, NAME> {
    type OptStorage = ReadOpt<'sys, T, NAME>;

    type NotStorage = ReadNot<'sys, NAME>;

    fn opt(self) -> Self::OptStorage {
        ReadOpt {
            storage: self.storage,
            bitset: self.bitset,
        }
    }

    fn not(self) -> Self::NotStorage {
        ReadNot {
            bitset: self.bitset,
        }
    }
}

impl<'sys, T: 'sys, const NAME: &'static str> SimpleStorage for &Read<'sys, T, NAME> {
    const NAME: &'static str = NAME;

    type Element = &'sys T;

    /// Type of the mask
    type Mask = &'sys BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        &self.bitset
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        match self.storage {
            ReadStorage::Vec(vec) => vec.get(entity as usize).unwrap().as_ref().unwrap(),
            ReadStorage::HashMap(map) => map.get(&Entity(entity)).unwrap(),
            ReadStorage::BTreeMap(map) => map.get(&Entity(entity)).unwrap(),
        }
    }
}

impl<'sys, T: 'sys, const NAME: &'static str> Storage for &Read<'sys, T, NAME> {
    type OptStorage = ReadOpt<'sys, T, NAME>;

    type NotStorage = ReadNot<'sys, NAME>;

    fn opt(self) -> Self::OptStorage {
        ReadOpt {
            storage: self.storage,
            bitset: self.bitset,
        }
    }

    fn not(self) -> Self::NotStorage {
        ReadNot {
            bitset: self.bitset,
        }
    }
}

impl<'sys, T: 'sys, const NAME: &'static str> Not for Read<'sys, T, NAME> {
    type Output = <Self as Storage>::NotStorage;

    fn not(self) -> Self::Output {
        <Self as Storage>::not(self)
    }
}

#[derive(Clone, Copy)]
pub struct ReadOpt<'sys, T: 'sys, const NAME: &'static str> {
    storage: ReadStorage<'sys, T>,
    bitset: &'sys BitSet,
}

impl<'sys, T: 'sys, const NAME: &'static str> SimpleStorage for ReadOpt<'sys, T, NAME> {
    const NAME: &'static str = NAME;

    type Element = Option<&'sys T>;

    /// Type of the mask
    type Mask = &'sys BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        self.bitset
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        match self.storage {
            ReadStorage::Vec(vec) => vec.get(entity as usize).unwrap().as_ref(),
            ReadStorage::HashMap(map) => map.get(&Entity(entity)),
            ReadStorage::BTreeMap(map) => map.get(&Entity(entity)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ReadNot<'sys, const NAME: &'static str> {
    bitset: &'sys BitSet,
}

impl<'sys, const NAME: &'static str> SimpleStorage for ReadNot<'sys, NAME> {
    const NAME: &'static str = NAME;

    type Element = ();

    /// Type of the mask
    type Mask = BitSetNot<&'sys BitSet>;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        BitSetNot(self.bitset)
    }

    unsafe fn get(&mut self, _entity: u32) -> Self::Element {
        ()
    }
}

pub enum WriteStorage<'sys, T> {
    Vec(&'sys mut Vec<Option<T>>),
    HashMap(&'sys mut FxHashMap<Entity, T>),
    BTreeMap(&'sys mut BTreeMap<Entity, T>),
}

pub struct Write<'sys, T: 'sys, const NAME: &'static str> {
    storage: WriteStorage<'sys, T>,
    bitset: &'sys BitSet,
}

impl<'sys: 'this, 'this, T: 'sys, const NAME: &'static str> Write<'sys, T, NAME> {
    pub fn new(storage: WriteStorage<'sys, T>, bitset: &'sys BitSet) -> Self {
        Self { storage, bitset }
    }
}

impl<'sys: 'this, 'this, T: 'sys, const NAME: &'static str> SimpleStorage
    for &'this mut Write<'sys, T, NAME>
{
    const NAME: &'static str = NAME;

    type Element = &'this mut T;

    /// Type of the mask
    type Mask = &'this BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        self.bitset
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        // This is **extremely** unsafe but I don't see another way of doing this
        let value = match &mut self.storage {
            WriteStorage::Vec(vec) => vec.get_mut(entity as usize).unwrap().as_mut().unwrap(),
            WriteStorage::HashMap(map) => map.get_mut(&Entity(entity)).unwrap(),
            WriteStorage::BTreeMap(map) => map.get_mut(&Entity(entity)).unwrap(),
        } as *mut T;

        &mut *value
    }
}

impl<'sys: 'this, 'this, T: 'sys, const NAME: &'static str> Storage
    for &'this mut Write<'sys, T, NAME>
{
    type OptStorage = WriteOptRef<'sys, 'this, T, NAME>;

    type NotStorage = WriteNot<'sys, NAME>;

    fn opt(self) -> Self::OptStorage {
        WriteOptRef {
            storage: &mut self.storage,
            bitset: self.bitset,
        }
    }

    fn not(self) -> Self::NotStorage {
        WriteNot {
            bitset: self.bitset,
        }
    }
}

pub struct WriteOpt<'sys, T: 'sys, const NAME: &'static str> {
    storage: WriteStorage<'sys, T>,
    bitset: &'sys BitSet,
}

impl<'sys, T: 'sys, const NAME: &'static str> SimpleStorage for WriteOpt<'sys, T, NAME> {
    const NAME: &'static str = NAME;

    type Element = Option<&'sys mut T>;

    /// Type of the mask
    type Mask = &'sys BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        self.bitset
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        // This is **extremely** unsafe but I don't see another way of doing this
        let value = match &mut self.storage {
            WriteStorage::Vec(vec) => vec.get_mut(entity as usize).unwrap().as_mut(),
            WriteStorage::HashMap(map) => map.get_mut(&Entity(entity)),
            WriteStorage::BTreeMap(map) => map.get_mut(&Entity(entity)),
        }
        .map(|val| val as *mut T);

        value.map(|val| &mut *val)
    }
}

pub struct WriteOptRef<'sys: 'this, 'this, T: 'sys, const NAME: &'static str> {
    storage: &'this mut WriteStorage<'sys, T>,
    bitset: &'sys BitSet,
}

impl<'sys: 'this, 'this, T: 'sys, const NAME: &'static str> SimpleStorage
    for WriteOptRef<'sys, 'this, T, NAME>
{
    const NAME: &'static str = NAME;

    type Element = Option<&'sys mut T>;

    /// Type of the mask
    type Mask = &'sys BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        self.bitset
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        // This is **extremely** unsafe but I don't see another way of doing this
        let value = match &mut self.storage {
            WriteStorage::Vec(vec) => vec.get_mut(entity as usize).unwrap().as_mut(),
            WriteStorage::HashMap(map) => map.get_mut(&Entity(entity)),
            WriteStorage::BTreeMap(map) => map.get_mut(&Entity(entity)),
        }
        .map(|val| val as *mut T);

        value.map(|val| &mut *val)
    }
}

pub struct WriteNot<'sys, const NAME: &'static str> {
    bitset: &'sys BitSet,
}

impl<'sys, const NAME: &'static str> SimpleStorage for WriteNot<'sys, NAME> {
    const NAME: &'static str = NAME;

    type Element = ();

    /// Type of the mask
    type Mask = BitSetNot<&'sys BitSet>;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        BitSetNot(self.bitset)
    }

    unsafe fn get(&mut self, _entity: u32) -> Self::Element {
        ()
    }
}

pub struct Entities<'sys>(&'sys BitSet);

impl<'sys> SimpleStorage for Entities<'sys> {
    const NAME: &'static str = "entities";

    type Element = Entity;

    /// Type of the mask
    type Mask = &'sys BitSet;

    /// Gets the mask
    fn mask(&self) -> Self::Mask {
        self.0
    }

    unsafe fn get(&mut self, entity: u32) -> Self::Element {
        Entity(entity)
    }
}

impl<'sys> Entities<'sys> {
    pub fn new(bitset: &'sys BitSet) -> Self {
        Self(bitset)
    }
    pub fn is_alive(&self, entity: Entity) {
        self.0.contains(entity.index());
    }
}
