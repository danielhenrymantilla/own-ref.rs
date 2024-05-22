// TODO: main crate docs.
#![warn(unsafe_op_in_unsafe_fn)]

#[macro_use]
extern crate extension_traits;

#[cfg(doctest)]
#[macro_use]
extern crate macro_rules_attribute;

#[cfg(test)]
extern crate self as own_ref;

#[macro_use]
mod utils;

pub use self::{
    own::OwnRef,
    slot::{Slot, slot, slots},
};

use self::{
    ඞ::*,
    prelude::*,
};

mod arities;

mod own;

pub
mod pin;

mod slot;

mod token;

pub
mod traits;

pub
mod prelude {
    #[doc(no_inline)]
    pub use {
        ::core::{
            future::Future,
            ops::Not as _,
            pin::{pin, Pin},
        },
        crate::{
            OwnRef,
            own_ref,
            slot::{slot, slots},
            traits::{FnOwn, MaybeUninitExt as _},
        },
        module::pin,
    };
    mod module {
        #![allow(warnings, clippy::all)]
        macro_rules! __ {() => ()} use __ as pin;
        pub use crate::*;
    }
}

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
            pin::{
                Pin,
            },
            ops::{
                Not as _,
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

#[cfg(any(test, doctest))]
mod tests;
