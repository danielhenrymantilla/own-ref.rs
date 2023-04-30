use super::*;

pub
struct OwnRef<'lt, T : ?Sized> {
    /// Mutating this field is unsound; it is only exposed for macro reasons and
    /// must be deemed private / `unsafe` to access otherwise.
    #[doc(hidden)] /** Not part of the public API */ pub
    r#unsafe: &'lt mut MD<T>,

    #[doc(hidden)] /** Not part of the public API */ pub
    _unsafe_to_construct: Unsafe,
}

impl<T : ?Sized> Drop for OwnRef<'_, T> {
    fn drop(&mut self)
    {
        unsafe {
            MD::drop(self.r#unsafe)
        }
    }
}

#[macro_export]
macro_rules! own {( $value:expr $(,)? ) => (
    OwnRef {
        r#unsafe: &mut MD::new($value),
        _unsafe_to_construct: unsafe { Unsafe::new() },
    }
)}

impl<'frame, T : ?Sized> OwnRef<'frame, T> {
    pub
    unsafe
    fn from_raw(
        r: &'frame mut MD<T>,
    ) -> OwnRef<'frame, T>
    {
        Self {
            r#unsafe: r,
            _unsafe_to_construct: Unsafe::new(),
        }
    }

    pub
    unsafe
    fn into_raw(
        self: OwnRef<'frame, T>,
    ) -> &'frame mut MD<T>
    {
        <*const _>::read(&MD::new(self).r#unsafe)
    }
}

impl<T : ?Sized> ::core::ops::DerefMut for OwnRef<'_, T> {
    fn deref_mut(&mut self)
      -> &mut T
    {
        impl<T : ?Sized> ::core::ops::Deref for OwnRef<'_, T> {
            type Target = T;
        
            fn deref(&self)
              -> &T
            {
                self.r#unsafe
            }
        }

        self.r#unsafe
    }
}
