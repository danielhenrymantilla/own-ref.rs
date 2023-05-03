use super::*;
use ::core::mem::ManuallyDrop;

pub
struct OwnRef<'slot, T : 'slot + ?Sized> {
    // Since `OwnRef` fields are technically exposed (for the macro to work)
    // we make it "more sound" by requiring an `unsafe`ty token:
    #[doc(hidden)] /** Not part of the public API. */ pub
    _unsafe_to_construct: Unsafe,
    // Alas, this technically isn't 100% sound if we wanted to be pedantic,
    // since users are technically capable of mutating the following field once
    // they have their hands on a legitimate instance.
    // Which is why the next field is named `unsafe`, to make it clearer that
    // mutating it is not safe.

    /// Not part of the public API.
    ///
    /// Moreover, mutating this field is unsound; it is only exposed for macro
    /// reasons and must be deemed private or `unsafe`-to-access otherwise.
    #[doc(hidden)] pub
    // We *need* this field to be:
    //   - Covariant in `<T>` (much like `T` and `Box<T>` are).
    //   - Compatible with lifetime extension shenanigans. This means:
    //       - either a `& mut? _`,
    //       - or a `* {const,mut} _` (since we can use `as _` casts)
    //       - or a braced struct thereof.
    //   - Able to carry (exclusive) `Write`-access provenance to the `T`.
    // Among the second point candidates:
    //   - the first point only allows for `&` or `*const`;
    //   - the third point only allows for `&mut` or `*{const,mut}`.
    // Thence `*const`.
    //
    // While we'd also love to have this field be:
    //   - non-null
    //   - assumed well-aligned, and unaliased (`&unique T`);
    // we can't have it all in stable Rust (on nightly Rust we could add the
    // non-null property through a braced struct newtype around `*const T`).
    r#unsafe: *const HackMD<PD<&'slot ()>, T>,

    #[doc(hidden)] /** Not part of the public API. */ pub
    _phantom: PD<OwnRefSemantics<'slot, T>>,
}

/// What is a `&'slot own T`, after all?
type OwnRefSemantics<'slot, T> = (
    //  1. it is a `&'slot mut` reference to its backing memory.
    &'slot mut [MU<u8>],
    //  2. it is an owned `T` instance.
    T,
    // Note: `2.` is only really needed with a `#[may_dangle]` drop impl
    // but we keep it nonetheless for the sake of documentation (to explain why
    // covariance in `T`, much like with `T` or `Box<T>`, is fine).
);

impl<'slot, T : ?Sized> Drop for OwnRef<'slot, T> {
    fn drop(&mut self)
    {
        let p: *mut T = self.r#unsafe as _;
        unsafe {
            <*mut T>::drop_in_place(p as _)
        }
    }
}

#[macro_export]
macro_rules! own_ref {( $value:expr $(,)? ) => ({
    let value = $value;
    #[allow(warnings, clippy::all, clippy::pedantic)] {
        OwnRef {
            _unsafe_to_construct: unsafe { $crate::ඞ::Unsafe::token() },
            _phantom: $crate::ඞ::PD,
            r#unsafe: (&mut
                                    $crate::ඞ::HackMD::<&(), _> {
                                        value:
                        $crate::ඞ::MD::new(value),
                                        _temporary: &::core::mem::drop(()),
                                    })
                                    // `DerefMut` coercion
                                    as &mut $crate::ඞ::HackMD::<$crate::ඞ::PD<&()>, _>
                    // go through `*mut` to avoid through-`&` provenance loss.
                    // (I'd have loved to use `addr_of_mut!` instead, but it
                    // purposedly rejects lifetime extension).
                    as *mut _
            ,
        }
    }
})}

impl<'slot, T : ?Sized> OwnRef<'slot, T> {
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
        yield_({ Slot::VACANT }.holding(value))
    }

    /// # Safety
    ///
    /// See [`ManuallyDrop::take()`].
    pub
    unsafe
    fn from_raw(
        r: &'slot mut ManuallyDrop<T>,
    ) -> OwnRef<'slot, T>
    {
        Self {
            _unsafe_to_construct: unsafe {
                // Safety: delegated to the caller
                Unsafe::token()
            },
            r#unsafe: ::core::ptr::addr_of_mut!(*HackMD::wrap_mut(r)),
            _phantom: <_>::default(),
        }
    }

    pub
    fn into_raw(
        self: OwnRef<'slot, T>,
    ) -> &'slot mut ManuallyDrop<T>
    {
        let p: *mut _ = MD::new(self).r#unsafe.cast_mut();
        HackMD::unwrap_mut(unsafe {
            &mut *p
        })
    }
}

#[macro_export]
macro_rules! unsize {( $e:expr $(,)? ) => (
    match $e { e => unsafe {
        $crate::OwnRef::from_raw($crate::OwnRef::into_raw(e) as _)
    }}
)}

impl<'slot, T : ?Sized> ::core::ops::DerefMut for OwnRef<'slot, T> {
    fn deref_mut(self: &'_ mut OwnRef<'slot, T>)
      -> &'_ mut T
    {
        impl<'slot, T : ?Sized> ::core::ops::Deref for OwnRef<'slot, T> {
            type Target = T;

            fn deref(self: &'_ OwnRef<'slot, T>)
              -> &'_ T
            {
                &unsafe { &*self.r#unsafe }.value
            }
        }

        HackMD::unwrap_mut(unsafe { &mut *self.r#unsafe.cast_mut() })
    }
}

mod autotraits {
    use super::*;

    unsafe
    impl<'slot, T : ?Sized> Send for OwnRef<'_, T>
    where
        OwnRefSemantics<'slot, T> : Send,
    {}

    unsafe
    impl<'slot, T : ?Sized> Sync for OwnRef<'_, T>
    where
        OwnRefSemantics<'slot, T> : Sync,
    {}

    impl<'slot, T : ?Sized> ::core::panic::UnwindSafe for OwnRef<'_, T>
    where
        OwnRefSemantics<'slot, T> : ::core::panic::UnwindSafe,
    {}

    // For this impl, the indirection is important, so we don't use
    // `OwnRefSemantics` (the true semantics are those with a `Box<T>`, but
    // we want to be `no_std`-friendly).
    impl<'slot, T : ?Sized> Unpin for OwnRef<'slot, T>
    where
        &'slot mut T : Unpin,
    {}
}


/// Helper type that allows keeping the type temporary-lifetime-infected while
/// avoiding encumbering the non-macro case with useless data.
///
/// The key observation/idea is that `HackMD<PD<_>, T>` and `MD<T>`
/// (and `T`) have the same layout (while still being `_`-lifetime-infected),
/// but thanks to a `Deref` hack (which can occur without hindering lifetime
/// extension), we can also convert
/// a `HackMD<&'temporary (), T>` into a `HackMD<PD<&'temporary ()>, T>`, which
/// gives us room to squeeze a `&drop(())` temporary into the whole expression.
///
/// Any resemblance with an online-editing Markdown website is accidental.
#[repr(C)]
pub
struct HackMD<Lifetime, T : ?Sized> {
    pub _temporary: Lifetime,
    pub value: MD<T>,
}

impl<__ : ?Sized, T : ?Sized> HackMD<PD<__>, T> {
    fn wrap_mut<'r>(
        r: &'r mut MD<T>,
    ) -> &'r mut HackMD<PD<__>, T>
    {
        unsafe {
            // Safety: same layout, thanks to `repr(C)`.
            ::core::mem::transmute(r)
        }
    }

    fn unwrap_mut<'r>(
        r: &'r mut HackMD<PD<__>, T>,
    ) -> &'r mut MD<T>
    {
        &mut r.value
    }
}

impl<'temporary, T : ?Sized>
    ::core::ops::DerefMut
for
    HackMD<&'temporary (), T>
{
    fn deref_mut(
        self: &'_ mut HackMD<&'temporary (), T>,
    ) -> &'_ mut HackMD<PD<&'temporary ()>, T>
    {
        impl<'temporary, T : ?Sized>
            ::core::ops::Deref
        for
            HackMD<&'temporary (), T>
        {
            type Target = HackMD<PD<&'temporary ()>, T>;

            fn deref(
                self: &'_ HackMD<&'temporary (), T>,
            ) -> &'_ HackMD<PD<&'temporary ()>, T>
            {
                // Should never need to be called.
                unimplemented!()
            }
        }

        HackMD::wrap_mut(&mut self.value)
    }
}
