//! Extra functionality of `&own T` references.
use ::core::fmt;
use std::mem;

use crate::OwnRef;

impl<'slot, T> OwnRef<'slot, T> {
    pub
    fn into_inner(
        self: OwnRef<'slot, T>,
    ) -> T
    {
        unsafe {
            <*mut T>::read(&mut **mem::ManuallyDrop::new(self))
        }
    }
}

impl<'slot, D, T : fmt::Debug> fmt::Debug for OwnRef<'slot, T, D> {
    fn fmt(
        self: &'_ OwnRef<'slot, T, D>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        (**self).fmt(f)
    }
}
