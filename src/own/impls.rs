//! Extra functionality of `&own T` references.
use ::core::fmt;

use crate::OwnRef;

impl<'slot, D, T : fmt::Debug> fmt::Debug for OwnRef<'slot, T, D> {
    fn fmt(
        self: &'_ OwnRef<'slot, T, D>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        (**self).fmt(f)
    }
}
