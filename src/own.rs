use super::*;

pub
struct OwnRef<'frame, T : 'frame + ?Sized> {
    /// Mutating this field is unsound; it is only exposed for macro reasons and
    /// must be deemed private / `unsafe` to access otherwise.
    ///
    #[doc(hidden)] /** Not part of the public API */ pub
    // We *need* this field to be:
    //   - Maximally variant. We can't be bivariant, but we know `_phantom` is
    //     not contravariant, so we don't need to be contravariant either.
    //   - Compatible with lifetime extension shenanigans. This means:
    //       - either a `& mut? _`,
    //       - or a `* {const,mut} _` (since we can use `as _` casts)
    //       - or a braced struct thereof.
    //   - Able to carry `Unique`-access provenance to the `T`.
    // We'd love to have this field also be:
    //   - non-null
    //   - assumed well-aligned, and unaliased (`&unique T`);
    // We can't have it all in stable Rust (on nightly Rust we could add the
    // non-null property through a braced struct newtype around `*const T`).
    r#unsafe: *const T,

    #[doc(hidden)] /** Not part of the public API */ pub
    _unsafe_to_construct: Unsafe,

    #[doc(hidden)] /** Not part of the public API */ pub
    _phantom: PhantomOwn<'frame, T>,

    #[doc(hidden)] /** Not part of the public API */ pub
    // Used for the soundness of the macro-construction.
    // Note: I'd love to be able to squash this into `_phantom`, or at least
    // some other ZST (_e.g._, `[&'frame (); 0]`), but I haven't found a way to:
    //   - get lifetime extension to kick in (so no `if false` shenanigans);
    //   - get the `'frame` to actually match the lifetime of a temporary, rather
    //     than it becoming unbounded (which, alas, happens inside a `[_; 0]`).
    _temporary_lt: &'frame (),
}

/// What is a `&'frame own T`, after all?
#[allow(type_alias_bounds)]
type PhantomOwn<'frame, T : ?Sized> = PD<(
    // 1: it is a `&'frame mut` reference to its backing memory.
    &'frame mut [MU<u8>],
    // 2: it is an owned `T` instance.
    T,
    // Note: `2.` is only needed with a `#[may_dangle]` drop impl
    // but we keep it nonetheless for the sake of documentation (to explain why
    // covariance in `T`, much like with `T` or `Box<T>`, is fine).
)>;

impl<T : ?Sized> Drop for OwnRef<'_, T> {
    fn drop(&mut self)
    {
        unsafe {
            self.r#unsafe.cast_mut().drop_in_place()
        }
    }
}

#[macro_export]
macro_rules! own_ref {( $value:expr $(,)? ) => ({
    #[allow(warnings, clippy::all, clippy::pedantic)] {
        OwnRef {
            _phantom: $crate::ඞ::nudge_type_inference(if false {
                [&$value; 0]
            } else {
                []
            }),
            r#unsafe: &mut *$crate::ඞ::MD::new($value) as *mut _ as *const _,
            _unsafe_to_construct: unsafe { Unsafe::new() },
            _temporary_lt: &::core::mem::drop(()),
        }
    }
})}

#[inline]
pub
fn nudge_type_inference<'unbounded, T : ?Sized>(
    _: [&'_ T; 0],
) -> PhantomOwn<'unbounded, T>
{
    PD
}

impl<'frame, T : ?Sized> OwnRef<'frame, T> {
    /// ```rust
    /// use ::own_ref::*;
    ///
    /// let x = OwnRef::with(String::from("…"), |o: OwnRef<'_, String>| {
    ///     assert_eq!(&o[..], "…");
    ///     42
    /// });
    /// assert_eq!(x, 42);
    /// ```
    pub
    fn with<R>(value: T, scope: impl FnOnce(OwnRef<'_, T>) -> R)
      -> R
    where
        T : Sized,
    {
        let yield_ = scope;
        yield_(own_ref!(value))
    }

    /// # Safety
    ///
    /// See [`ManuallyDrop::take()`][MD::take].
    pub
    unsafe
    fn from_raw(
        r: &'frame mut MD<T>,
    ) -> OwnRef<'frame, T>
    {
        Self {
            r#unsafe: (&mut **r) as *mut _,
            _unsafe_to_construct: Unsafe::new(),
            _phantom: <_>::default(),
            _temporary_lt: &(),
        }
    }

    pub
    fn into_raw(
        self: OwnRef<'frame, T>,
    ) -> &'frame mut MD<T>
    {
        #![allow(clippy::transmute_ptr_to_ref)] // `?Sized`.
        unsafe {
            ::core::mem::transmute::<*const T, _>(MD::new(self).r#unsafe)
        }
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
                unsafe { &*self.r#unsafe }
            }
        }

        unsafe { &mut *self.r#unsafe.cast_mut() }
    }
}

mod autotraits {
    use super::*;

    unsafe
    impl<T : ?Sized> Send for OwnRef<'_, T>
    where
        T : Send,
    {}

    unsafe
    impl<T : ?Sized> Sync for OwnRef<'_, T>
    where
        T : Sync,
    {}

    impl<T : ?Sized> ::core::panic::UnwindSafe for OwnRef<'_, T>
    where
        T : ::core::panic::UnwindSafe,
    {}

    impl<'frame, T : ?Sized> Unpin for OwnRef<'frame, T>
    where
        &'frame mut T : Unpin,
    {}
}
