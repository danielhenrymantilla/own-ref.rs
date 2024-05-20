#[cfg(doc)]
use crate::pin::DropFlags;

use super::*;
use ::core::mem::ManuallyDrop;

mod impls;

pub
struct OwnRef<
    'slot,
    T : 'slot + ?Sized,
    DropFlags : 'static = pin::DropFlags::No,
> {
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
    //     (And even beyond `T` in the case of `DropFlags::Yes`.)
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
    _semantics: PD<OwnRefSemantics<'slot, T>>,

    // Regarding `DropFlags`, we just want an *implicit* `: 'static`.
    // And now that we are at it, we may as well introduce an implicit
    // `T : 'slot` as well.
    #[doc(hidden)] /** Not part of the public API. */ pub
    _drop_flags_marker: PD<fn() -> (&'static DropFlags, &'slot T)>,
}

/// What is a `&'slot own T`, after all?
type OwnRefSemantics<'slot, T> = (
    //  1. it is a `&'slot mut` reference to its backing memory.
    &'slot mut [MU<u8>],
    //  2. it is an owned `T` instance (granted, behind indirection, so we'll
    // need to adjust our impl of `Unpin` (the only indirection-sensitive trait)
    // accordingly).
    T,
    // Note: `2.` is only really needed with a `#[may_dangle]` drop impl
    // but we keep it nonetheless for the sake of documentation (to explain why
    // covariance in `T`, much like with `T` or `Box<T>`, is fine).
);

impl<'slot, T : ?Sized, DropFlags> Drop for OwnRef<'slot, T, DropFlags> {
    fn drop(&mut self)
    {
        if ::core::mem::needs_drop::<T>() {
            // Don't forget to clear the drop flag when marked to do so.
            if PartialEq::eq(
                &::core::any::TypeId::of::<DropFlags>(),
                &::core::any::TypeId::of::<pin::DropFlags::Yes>(),
            )
            {
                // Safety: `.unsafe` is a pointer to the `.value`
                // field of a `ManualOption<T>`, with exclusive write
                // provenance over it all.
                let align = ::core::mem::align_of_val::<T>(self);
                let is_some: *mut bool =
                    unsafe {
                        (self.r#unsafe as *mut u8)
                            .sub(align)
                    }
                    .cast()
                ;
                unsafe {
                    *is_some = false;
                }
            }
            unsafe {
                // Safety: per the whole design of this whole crate:
                // the pointer is valid, well-aligned, with exclusive write
                // provenance over `T`, and the `T` itself won't be accessed
                // as such (_e.g._, won't be dropped) after this point.
                <*mut T>::drop_in_place(self.r#unsafe as _)
            }
        }
    }
}

#[macro_export]
macro_rules! own_ref {( $value:expr $(,)? ) => ({
    let value = $value;
    #[allow(warnings, clippy::all, clippy::pedantic)] {
        // Safety: we construct a `&mut MD<T>` temporary and pass a pointer to it
        // to this `OwnRef` literal construction. Since the raw pointer erases
        // the lifetime of this temporary, we also create a `&()` temporary
        // alongside this one (with, thus, undistinguishables temporary
        // lifetimes), and manage, for that one, to keep hold of its lifetime
        // marker/parameter all the way down to the final construction, so that
        // the resulting instance is properly temporary-lifetime infected to
        // prevent usage beyond the scope where it is defined.
        //
        // The whole `HackMD` layer is then just there to hide the `&()` so as
        // to unify with `OwnRef`s constructed otherwise (_e.g._, from a `Slot`
        // or the `with()` scoped constructor).
        OwnRef::<'_, _, $crate::pin::DropFlags::No> {
            _unsafe_to_construct: unsafe { $crate::ඞ::Unsafe::token() },
            r#unsafe:
                // main temporary
                (&mut $crate::ඞ::HackMD::<&(), _> {
                    value: $crate::ඞ::MD::new(value),
                    // extra temporary whose lifetime is not erased.
                    _temporary: &::core::mem::drop(()),
                })
                // `DerefMut` coercion (to yeet the pointer to the extra
                // temporary into `PhantomData` oblivion (but not its lifetime))
                as &mut $crate::ඞ::HackMD::<$crate::ඞ::PD<&()>, _>

                // go through `*mut` to avoid through-`&` provenance loss.
                // (I'd have loved to use `addr_of_mut!` instead, but it
                // purposely rejects lifetime extension).
                as *mut _
            ,
            _semantics: $crate::ඞ::PD,
            _drop_flags_marker: $crate::ඞ::PD,
        }
    }
})}

impl<'slot, T> OwnRef<'slot, T> {
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
    {
        let yield_ = scope;
        yield_(slot().holding(value))
    }
}

impl<'slot, T : ?Sized, D> OwnRef<'slot, T, D> {
    /// Construct a [`Self`] out of a
    /// <code>&\'slot mut [ManuallyDrop]\<T\></code>.
    ///
    ///   - (Consider the arg pair as acting as one).
    ///
    /// # Safety
    ///   0. Casting the ptr to a `&'slot mut ManuallyDrop<T>` must be sound.
    ///
    ///   1. Since the resulting pointer has ownership over the pointee `T`,
    ///      _i.e._, since `T` is to be dropped by `Self`, then
    ///      [`ManuallyDrop::take()`] (and/or [`ManuallyDrop::drop()`])
    ///      requirements fully apply.
    ///
    ///   2. `D` ought not to be [`pin::DropFlags::Yes`].
    ///
    ///      If it is, then careful with variance! Also, `ptr` must be pointing
    ///      to the `.value` field of a [`pin::ManualOption`], with exclusive
    ///      write provenance over the whole `ManualOption`.
    ///
    ///        - (currently that field is not exposed at all publicly since it
    ///          is a very finicky requirement).
    #[inline(always)]
    pub
    unsafe
    fn from_raw(
        ptr: *mut ManuallyDrop<T>,
        _you_can_use_this_to_bound_the_lifetime: [&'slot (); 0],
    ) -> OwnRef<'slot, T, D>
    {
        Self {
            _unsafe_to_construct: unsafe {
                // Safety: delegated to the caller
                Unsafe::token()
            },
            r#unsafe: unsafe {
                // Safety: same layout (pointer to `?Sized`).
                // (this is less error-prone than using casts since it avoids
                // accidentally affecting provenance)
                ::core::mem::transmute(ptr)
            },
            _semantics: <_>::default(),
            _drop_flags_marker: <_>::default(),
        }
    }

    #[inline(always)]
    pub
    fn into_raw(
        self: OwnRef<'slot, T, D>,
    ) -> (*mut ManuallyDrop<T>, [&'slot (); 0])
    {
        (
            unsafe {
                // Safety: same layout (pointer to `?Sized`)
                ::core::mem::transmute(self)
            },
            [],
        )
    }
}

#[macro_export]
macro_rules! unsize {( $e:expr $(,)? ) => (
    // Safety: `from_raw()` and `into_raw()` are inverses of one another,
    // so semantically this is fine.
    // The point of doing this is that it creates a `ptr` place where an unsized
    // coercion can occur to widen it.
    // (`from_raw` (and the rest of the `OwnRef` machinery) is resilient to
    // having wide pointers around.)
    match $crate::OwnRef::into_raw($e) { (ptr, lt) => unsafe {
        $crate::OwnRef::from_raw(ptr, lt)
    }}
)}

impl<'slot, T : ?Sized, D> ::core::ops::DerefMut for OwnRef<'slot, T, D> {
    fn deref_mut(self: &'_ mut OwnRef<'slot, T, D>)
      -> &'_ mut T
    {
        // We needn't worry about provenance shrinkage since these are
        // short-lived (`'_`) {nested/re}borrowing operations which only care
        // about accessing the underlying `T`.
        impl<'slot, T : ?Sized, D> ::core::ops::Deref for OwnRef<'slot, T, D> {
            type Target = T;

            fn deref(self: &'_ OwnRef<'slot, T, D>)
              -> &'_ T
            {
                &unsafe {
                    // Safety: constructed from a valid reference
                    &*self.r#unsafe
                }.value
            }
        }

        HackMD::unwrap_mut(unsafe {
            // Safety: constructed from a valid reference
            &mut *self.r#unsafe.cast_mut()
        })
    }
}

mod autotraits {
    use super::*;

    unsafe
    impl<'slot, T : ?Sized, D> Send for OwnRef<'_, T, D>
    where
        OwnRefSemantics<'slot, T> : Send,
    {}

    unsafe
    impl<'slot, T : ?Sized, D> Sync for OwnRef<'_, T, D>
    where
        OwnRefSemantics<'slot, T> : Sync,
    {}

    impl<'slot, T : ?Sized, D> ::core::panic::UnwindSafe for OwnRef<'_, T, D>
    where
        OwnRefSemantics<'slot, T> : ::core::panic::UnwindSafe,
    {}

    impl<'slot, T : ?Sized, D> ::core::panic::RefUnwindSafe for OwnRef<'_, T, D>
    where
        OwnRefSemantics<'slot, T> : ::core::panic::RefUnwindSafe,
    {}

    // For this impl, the indirection is important, so we don't use
    // `OwnRefSemantics` (the true semantics are those of a `Box<T>`, but
    // we want to be `no_std`-friendly).
    impl<'slot, T : ?Sized, D> Unpin for OwnRef<'slot, T, D>
    where
        &'slot mut T : Unpin,
    {}
}


/// Helper type that allows keeping the type temporary-lifetime-infected,
/// without encumbering the non-macro case with useless data.
///
/// The key observation/idea is that `HackMD<PD<_>, T>` and `MD<T>` (and `T`)
/// have the same layout (whilst still being `_`-lifetime-infected),
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
