#![feature(coerce_unsized)]
#![warn(unsafe_op_in_unsafe_fn)]

#[macro_use]
extern crate extension_traits;

#[cfg(doctest)]
#[macro_use]
extern crate macro_rules_attribute;

#[macro_use]
mod utils;

pub use self::{
    own::OwnRef,
    slot::{MaybeUninitExt, Slot, slot, slots},
};

use self::ඞ::*;

mod own;
mod slot;
mod token;

#[doc(hidden)] /** Not part of the public API */ pub
mod ඞ {
    pub use {
        ::core::{
            marker::{
                PhantomData as PD,
            },
            mem::{
                ManuallyDrop as MD,
                MaybeUninit as MU,
            },
        },
        crate::{
            own::{
                HackMD,
            },
            token::Unsafe,
        },
    };
}

#[cfg(FALSE)]
impl<'frame, T : ?Sized, U : ?Sized>
    ::core::ops::CoerceUnsized<OwnRef<'frame, U>>
for
    OwnRef<'frame, T>
where
    &'frame mut MD<T> : ::core::ops::CoerceUnsized<&'frame mut MD<U>>,
{}

#[cfg(any(test, doctest))]
mod tests;
