#![feature(coerce_unsized)]

pub use self::{
    own::OwnRef,
    slot::{Slot, slot, slots},
    token::Unsafe,
};

use ::core::mem::{
    ManuallyDrop as MD,
    MaybeUninit as MU,
};

mod own;
mod slot;
mod token;

#[cfg(FALSE)]
impl<'frame, T : ?Sized, U : ?Sized>
    ::core::ops::CoerceUnsized<OwnRef<'frame, U>>
for
    OwnRef<'frame, T>
where
    &'frame mut MD<T> : ::core::ops::CoerceUnsized<&'frame mut MD<U>>,
{}

// #[cfg(test)]
mod tests;

#[macro_export]
macro_rules! unsize {( $e:expr $(,)? ) => (
    match $e { e => unsafe {
        $crate::OwnRef::from_raw($crate::OwnRef::into_raw(e) as _)
    }}
)}
