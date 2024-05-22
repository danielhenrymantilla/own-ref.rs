//! Traits designed to interact with [`OwnRef`][crate::OwnRef]s.
//!
//! And also some minor convenience extension traits.

mod any;

pub use fn_own::{FnOwn, FnOwnRet};
mod fn_own;

#[doc(inline)]
pub use crate::slot::MaybeUninitExt;
