//! # Join
//! Joining allows taking multiple [`Read`] and [`Write`] accesses and joining
//! them together in order to iterate over them. This is **heavily** inspired by
//! this mechanism in [specs](https://github.com/amethyst/specs) and mostly ripped
//! off of [their code](https://github.com/amethyst/specs/blob/master/src/join/mod.rs)

use hibitset::{BitIter, BitSetAnd, BitSetLike};
use tuple_utils::Split;

use crate::{storage::SimpleStorage, Entity};

/// The purpose of the `Join` trait is to provide a way
/// to access multiple storages at the same time with
/// the merged bit set.
///
/// Joining component storages means that you'll only get values where
/// for a given entity every storage has an associated component.
pub trait Join {
    /// Type of joined components.
    type Type;

    /// Type of joined storages.
    type Value;

    /// Type of joined bit mask.
    type Mask: BitSetLike;

    /// Create a joined iterator over the contents.
    fn join(self) -> JoinIter<Self>
    where
        Self: Sized,
    {
        JoinIter::new(self)
    }

    /// Open this join by returning the mask and the storages.
    ///
    /// # Safety
    ///
    /// This is unsafe because implementations of this trait can permit
    /// the `Value` to be mutated independently of the `Mask`.
    /// If the `Mask` does not correctly report the status of the `Value`
    /// then illegal memory access can occur.
    unsafe fn open(self) -> (Self::Mask, Self::Value);

    /// Get a joined component value by a given index.
    ///
    /// # Safety
    ///
    /// * A call to `get` must be preceded by a check if `id` is part of
    ///   `Self::Mask`
    /// * The implementation of this method may use unsafe code, but has no
    ///   invariants to meet
    unsafe fn get(value: &mut Self::Value, id: u32) -> Self::Type;
}

impl<S> Join for S
where
    S: SimpleStorage,
{
    type Type = <S as SimpleStorage>::Element;

    type Value = S;

    type Mask = <S as SimpleStorage>::Mask;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (<Self as SimpleStorage>::mask(&self), self)
    }

    unsafe fn get(value: &mut Self::Value, id: u32) -> Self::Type {
        <S as SimpleStorage>::get(value, id)
    }
}

/// `JoinIter` is an `Iterator` over a group of `Storages`.
#[must_use]
pub struct JoinIter<J: Join> {
    keys: BitIter<J::Mask>,
    values: J::Value,
}

impl<J: Join> JoinIter<J> {
    /// Create a new join iterator.
    pub fn new(j: J) -> Self {
        // SAFETY: We do not swap out the mask or the values, nor do we allow it by
        // exposing them.
        let (keys, values) = unsafe { j.open() };
        JoinIter {
            keys: keys.iter(),
            values,
        }
    }

    /// Allows getting joined values for specific entity.
    pub fn get(&mut self, entity: Entity) -> Option<J::Type> {
        if self.keys.contains(entity.index()) {
            // SAFETY: the mask (`keys`) is checked as specified in the docs of `get`.
            Some(unsafe { J::get(&mut self.values, entity.index()) })
        } else {
            None
        }
    }

    /// Allows getting joined values for specific raw index.
    ///
    /// The raw index for an `Entity` can be retrieved using `Entity::id`
    /// method.
    ///
    /// As this method operates on raw indices, there is no check to see if the
    /// entity is still alive, so the caller should ensure it instead.
    pub fn get_unchecked(&mut self, index: u32) -> Option<J::Type> {
        if self.keys.contains(index) {
            // SAFETY: the mask (`keys`) is checked as specified in the docs of `get`.
            Some(unsafe { J::get(&mut self.values, index) })
        } else {
            None
        }
    }
}

impl<J: Join> std::iter::Iterator for JoinIter<J> {
    type Item = J::Type;

    fn next(&mut self) -> Option<J::Type> {
        // SAFETY: since `idx` is yielded from `keys` (the mask), it is necessarily a
        // part of it. Thus, requirements are fulfilled for calling `get`.
        self.keys
            .next()
            .map(|idx| unsafe { J::get(&mut self.values, idx) })
    }
}

impl<J: Join> Clone for JoinIter<J>
where
    J::Mask: Clone,
    J::Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            values: self.values.clone(),
        }
    }
}

macro_rules! define_open {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<$($from,)*> Join for ($($from),*,)
            where $($from: Join),*,
                  ($(<$from as Join>::Mask,)*): BitAnd,
        {
            type Type = ($($from::Type),*,);
            type Value = ($($from::Value),*,);
            type Mask = <($($from::Mask,)*) as BitAnd>::Value;
            #[allow(non_snake_case)]

            // SAFETY: While we do expose the mask and the values and therefore would allow swapping them,
            // this method is `unsafe` and relies on the same invariants.
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                let ($($from,)*) = self;
                let ($($from,)*) = ($($from.open(),)*);
                (
                    ($($from.0),*,).and(),
                    ($($from.1),*,)
                )
            }

            // SAFETY: No invariants to meet and `get` is safe to call as the caller must have checked the mask,
            // which only has a key that exists in all of the storages.
            #[allow(non_snake_case)]
            unsafe fn get(v: &mut Self::Value, i: u32) -> Self::Type {
                let &mut ($(ref mut $from,)*) = v;
                ($($from::get($from, i),)*)
            }
        }

    }
}

define_open! {A}
define_open! {A, B}
define_open! {A, B, C}
define_open! {A, B, C, D}
define_open! {A, B, C, D, E}
define_open! {A, B, C, D, E, F}
define_open! {A, B, C, D, E, F, G}
define_open! {A, B, C, D, E, F, G, H}
define_open! {A, B, C, D, E, F, G, H, I}
define_open! {A, B, C, D, E, F, G, H, I, J}
define_open! {A, B, C, D, E, F, G, H, I, J, K}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R}

/// `BitAnd` is a helper method to & bitsets together resulting in a tree.
pub trait BitAnd {
    /// The combined bitsets.
    type Value: BitSetLike;
    /// Combines `Self` into a single `BitSetLike` through `BitSetAnd`.
    fn and(self) -> Self::Value;
}

/// This needs to be special cased
impl<A> BitAnd for (A,)
where
    A: BitSetLike,
{
    type Value = A;

    fn and(self) -> Self::Value {
        self.0
    }
}

macro_rules! bitset_and {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<$($from),*> BitAnd for ($($from),*)
            where $($from: BitSetLike),*
        {
            type Value = BitSetAnd<
                <<Self as Split>::Left as BitAnd>::Value,
                <<Self as Split>::Right as BitAnd>::Value
            >;

            fn and(self) -> Self::Value {
                let (l, r) = self.split();
                BitSetAnd(l.and(), r.and())
            }
        }
    }
}

bitset_and! {A, B}
bitset_and! {A, B, C}
bitset_and! {A, B, C, D}
bitset_and! {A, B, C, D, E}
bitset_and! {A, B, C, D, E, F}
bitset_and! {A, B, C, D, E, F, G}
bitset_and! {A, B, C, D, E, F, G, H}
bitset_and! {A, B, C, D, E, F, G, H, I}
bitset_and! {A, B, C, D, E, F, G, H, I, J}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M, N}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}
