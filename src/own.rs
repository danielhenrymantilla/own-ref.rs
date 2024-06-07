#[cfg(doc)]
use crate::pin::DropFlags;

use {
    ::core::{
        mem::ManuallyDrop,
    },
    super::{
        *,
    },
};

mod impls;

/// `&'slot own T`.
// TODO: main crate docs.
pub
struct OwnRef<
    'slot,
    T : 'slot + ?Sized,
    DropFlags : 'static = pin::DropFlags::No,
> {
    // Since `OwnRef` fields are technically exposed (for the macro to work)
    // we make it "more sound" by requiring an `unsafe`ty token:
    #[doc(hidden)] /** Not part of the public API. */ pub
    _ඞunsafe_to_construct: Unsafe,
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
    //       - EDIT: actually, see `_non_covariant_in_case_of_drop_flags`.
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
    _ඞsemantics: PD<OwnRefSemantics<'slot, T>>,

    // Regarding `DropFlags`, we just want an *implicit* `: 'static`.
    // And now that we are at it, we may as well introduce an implicit
    // `T : 'slot` as well.
    #[doc(hidden)] /** Not part of the public API. */ pub
    _ඞdrop_flags_marker: PD<fn() -> (&'static DropFlags, &'slot T)>,

    // A note about covariance: an `&'_ own T`, that is, an `OwnRef<'_, T>`,
    // i.e., an `OwnRef<'_, T, DropFlags::No>`, can, conceptually, be perfectly
    // well covariant (despite the `DerefMut`), much like a `Box` does:
    // ownership is strong enough of a restriction to avoid the unsoundness that
    // stems from borrowed/externally-witnessable covariant mutable access
    // (_e.g._, that of `&mut T`, or `&Cell<T>`).
    //
    // However, we do have a problem in the `DropFlags::Yes` case (the design
    // which has been used to become `Pin`-constructible). Indeed, the presence
    // of these drop flags is making our `OwnRef<'_, T, DropFlags::Yes>` act
    // more like a `&mut Option<T>` than like a `&mut ManuallyDrop<T>`.
    //
    // And this is a problem, since an `Option<T>` does very much have `T`-using
    // drop glue (the whole point of the `DropFlags::Yes` design!).
    //
    // And if the backing storage ceases to be a dummy bag-of-bytes entity, and
    // is now an entity capable of dropping a typed `T` as such (even though
    // this is only supposed to happen in the unlikely/silly case of the
    // `Pin<OwnRef<…>>` owner having `mem::forget()`ten it or such), then we
    // very much no longer have the necessary *full, detached-from-parent
    // ownership* which the mutable-yet-covariant handle requires for soundness.
    //
    // That is, `OwnRef<'_, T, DropFlags::Yes>` must very much *not* be
    // covariant over `T`, lest unsoundness ensue.
    // See `fn guard_against_covariance_if_drop_flags()` for a demo.
    //
    // And, alas, there is no way for the specific choice of a generic
    // parameter (here, `D`), to affect (here, reduce) the variance of another
    // generic parameter (here, `T`). The intuitive `D::Gat<T>` type is
    // currently unconditionally invariant...
    #[doc(hidden)] /** Not part of the public API. */ pub
    _ඞnon_covariant_in_case_of_drop_flags: PD<fn(&T)>,
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

impl<'slot, T> OwnRef<'slot, T> {
    /// Perform a "deref-move" operation akin to `*` on `Box`es.
    ///
    /// Same API as `OwnRef::into_inner()`, but with some more _panache_.
    pub
    fn deref_move(
        self: OwnRef<'slot, T>,
    ) -> T
    {
        unsafe {
            <*mut T>::read(&mut **MD::new(self))
        }
    }
}

/// Main/most useful [`OwnRef`] constructor.
///
/// It works very similarly to [`pin!`], but producing [`OwnRef`]s instead.
///
/// ## Syntax
///
///   - `own_ref!(<expr>)`, which infers the type of `<expr>` to output;
///
///     ```rust
///     # const _: &str = stringify! {
///     let own = own_ref!(some_variable);
///     let own = own_ref!(some_call());
///     # };
///     ```
///
///   - `own_ref!(: <type> = <expr>)`, which nudges type inference to pick `<type>`.
///
///     ```rust
///     # const _: &str = r#"
///     let own = own_ref!(: i32 = 42);
///     let own = own_ref!(: _ = value…); // same as `own_ref!(value…)`
///     # "#;
///     ```
///
///     Mostly useful when chaining the `own_ref!()` construction with method
///     calls, such as
///
///     ```rust
///     # use ::own_ref::prelude::*;
///     own_ref!(: String = "…".into())
///         .downcast::<bool>()
///         .unwrap_err()
///     # ;
///     ```
///
/// ## Examples
///
/// That is, it is very useful when working with small-scoped [`OwnRef`]s,
///
///   - be it when inlined within a function call, as with:
///
///     ```rust
///     //! Who needs `#![feature(unsized_fn_params)]`?
///
///     use ::own_ref::prelude::*;
///
///     fn demo(f: OwnRef<'_, dyn FnOwn<(), Ret = ()>>) {
///         f.call_ownref_0()
///     }
///
///     let captured = String::from("not copy");
///     demo(own_ref!(|| drop(captured))); // 👈 inlined usage!
///     ```
///
/// [1.79.0]: https://releases.rs/docs/1.79.0/
///
///   - or in small-ish scopes (using Rust ≥ [1.79.0] is then recommended):
///
///     ```rust
///     use ::own_ref::prelude::*;
///     # let some_condition = true;
///     # let some_mutex = ::std::sync::Mutex::new(());
///     let cleanup: OwnRef<'_, dyn FnOwn<(), Ret = ()>> = if some_condition {
///         let lock_guard = some_mutex.lock().unwrap();
///         // ...
///         own_ref!(move || drop(lock_guard))
///     } else {
///         own_ref!(|| ())
///     };
///     // stuff... (in the same scope)
///     // ...
///     // Eventually:
///     cleanup.call_ownref_0()
///     ```
///
/// Notice how, in both of these examples, we have taken advantage of the
/// built-in [unsizing][unsize!] capabilities of [`own_ref!`] (in this instance,
/// to <code>dyn [FnOwn]\<…\></code>).
///
/// ## Misusage
///
/// Be aware, however, that [`own_ref!`] operates in a manner very similar to
/// that of [`pin!`], which involves [temporary lifetime extension shenanigans](
/// https://doc.rust-lang.org/1.58.1/reference/destructors.html#temporary-lifetime-extension)
/// (_c.f._ the [`pin!`] docs for potentially more info about it).
///
/// The following, for instance, fails to compile:
///
/// ```rust ,compile_fail
/// use ::own_ref::prelude::*;
///
/// let o = Some(42).map(|x| own_ref!(x));
/// dbg!(o);
/// ```
///
/// with:
///
/// ```rust ,ignore
/// # r#"
/// error[E0515]: cannot return value referencing temporary value
///  --> src/own.rs:200:26
///   |
/// 8 | let o = Some(42).map(|x| own_ref!(x));
///   |                          ^^^^^^^^^^^
///   |                          |
///   |                          returns a value referencing data owned by the current function
///   |                          temporary value created here
///   |
///   = note: this error originates in the macro `own_ref`
/// # "#
/// ```
///
/// ## Alternatives
///
/// If you run into this problem, consider using the alternative constructors:
///
///   - either [`slot().holding()`][crate::slot()];
///   - or the [`OwnRef::with()`] scoped API.
///
/// (these, however, do not feature built-in unsizing, so they might require
/// explicit calls to [`unsize!`].)
#[macro_export]
macro_rules! own_ref {( : $T:ty = $value:expr $(,)? ) => ({
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
        OwnRef::<'_, $T, $crate::pin::DropFlags::No> {
            _ඞunsafe_to_construct: unsafe { $crate::ඞ::Unsafe::token() },
            r#unsafe:
                // main temporary
                (&mut $crate::ඞ::HackMD::<&(), $T> {
                    value: $crate::ඞ::MD::new(value),
                    // extra temporary whose lifetime is not erased.
                    _temporary: &::core::mem::drop(()),
                })
                // `DerefMut` coercion (to yeet the pointer to the extra
                // temporary into `PhantomData` oblivion (but not its lifetime))
                as &mut $crate::ඞ::HackMD::<$crate::ඞ::PD<&()>, $T>

                // go through `*mut` to avoid through-`&` provenance loss.
                // (I'd have loved to use `addr_of_mut!` instead, but it
                // purposely rejects lifetime extension).
                as *mut $crate::ඞ::HackMD::<$crate::ඞ::PD<&()>, $T>
            ,
            _ඞsemantics: $crate::ඞ::PD,
            _ඞdrop_flags_marker: $crate::ඞ::PD,
            _ඞnon_covariant_in_case_of_drop_flags: $crate::ඞ::PD,
        }
    }
});

(
    $value:expr $(,)?
) => (
    $crate::own_ref! { : _ = $value }
)}

impl<'slot, T> OwnRef<'slot, T> {
    /// Low-level [`OwnRef`] construction.
    ///
    /// An <code>[OwnRef]\<\'slot, T\></code>, at least, one with
    /// [`No`][crate::pin::DropFlags::No] [`DropFlags`][crate::pin::DropFlags]
    /// attached, is "merely" a "glorified"
    /// <code>&\'slot mut [ManuallyDrop]\<T\></code>, with an automatic
    /// [`ManuallyDrop::drop()`] invocation engrained into its [`Drop`] glue,
    /// and thus, _raison d'être_.
    ///
    ///   - From there, it grows to become way more than that, thanks to its
    ///     [`unsizing`][crate::unsize!] capabilities, which in turn subsume
    ///     whole language features such as `#![feature(unsized_fn_params)]`, or
    ///     even `#![feature(unsized_rvalues)]` altogether.
    ///
    /// It thus makes sense for such a basic and quintessential construction to
    /// be available.
    ///
    /// Do note that the [`Pin`-related APIs][mod@crate::pin] and types, such as
    /// <code>[OwnRef]\<\'\_, T, [DropFlags::Yes]\></code>, are more involved
    /// and subtle than this, with (raw) pointer _provenance_ playing an
    /// important role. Try to steer away from `unsafe`ly constructing that type.
    ///
    /// [DropFlags::Yes]: crate::pin::DropFlags::Yes
    ///
    /// # Safety
    ///
    /// Calling this returns a handle which, ultimately, calls
    /// [`ManuallyDrop::drop()`] (or [`ManuallyDrop::take()`] if calling
    /// [`OwnRef::deref_move()`]), so necessarily, the safety requirements and
    /// _caveats_ of these [`ManuallyDrop`] APIs apply.
    ///
    /// Good news is, they also suffice.
    pub
    unsafe
    fn from_ref_unchecked(r: &'slot mut ManuallyDrop<T>)
      -> OwnRef<'slot, T, crate::pin::DropFlags::No>
    {
        unsafe {
            // Safety: mainly inherited from the caller's narrow contract.
            // Notice `D = DropFlags::No`
            Self::from_raw(<*mut _>::cast(r), [])
        }
    }
}

impl<T> OwnRef<'_, T> {
    /// Simple, albeit limited, [`OwnRef`] constructor, through a scoped API.
    ///
    /// Using [`own_ref!`] will, most of the time, result in a more flexible and
    /// pleasant API.
    ///
    /// ## Example
    ///
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
        ptr: *mut T,
        _you_can_use_this_to_bound_the_lifetime: [&'slot (); 0],
    ) -> OwnRef<'slot, T, D>
    {
        // check that `D` is one of `No`, `Yes`.
        {
            use ::core::any::TypeId;
            use crate::pin::DropFlags::*;
            let tid = TypeId::of::<D>();
            match () {
                _case if tid == TypeId::of::<Yes>() => {},
                _case if tid == TypeId::of::<No>() => {},
                _default => panic!(
                    "instantiated `OwnRef::<_, D>::from_raw()` with D = {tid:?} not in `DropFlags`",
                ),
            }
        }
        Self {
            _ඞunsafe_to_construct: unsafe {
                // Safety: delegated to the caller
                Unsafe::token()
            },
            r#unsafe: unsafe {
                // Safety: same layout (pointer to `?Sized`).
                // (this is less error-prone than using casts since it avoids
                // accidentally affecting provenance)
                ::core::mem::transmute(ptr)
            },
            _ඞsemantics: <_>::default(),
            _ඞdrop_flags_marker: <_>::default(),
            _ඞnon_covariant_in_case_of_drop_flags: <_>::default(),
        }
    }

    /// Converts the [`OwnRef`] back into its constituent raw pointer,
    /// disabling the [`Drop`] glue, and whatnot.
    ///
    /// The returned pair is conceptually equivalent to a
    /// <code>\&\'slot [ManuallyDrop]\<T\></code>, but the usage of a raw
    /// pointer avoids shrinking provenance of the pointer, which matters
    /// when `D` is [`DropFlags::Yes`][crate::pin::DropFlags].
    #[inline(always)]
    pub
    fn into_raw(
        self: OwnRef<'slot, T, D>,
    ) -> (*mut T, [&'slot (); 0])
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

/// Perform an [`Unsize`][Unsize] coërcion on an owned [`OwnRef`].
///
/// If <code>T : [Unsize]\<dyn Trait + …\></code>, and
/// <code>o: [OwnRef]\<\'\_, T\></code>, then <code>[unsize!]\(o\)</code>
/// can become an <code>[OwnRef]\<'_, dyn Trait + …\></code>.
///
/// But be aware that the [`own_ref!`] macro itself already bundles `unsize!`
/// semantics (and "redundantly" calling
/// <code>[unsize!]\([own_ref!]\(…\)\)</code> will actually mess up the
/// temporary lifetime extension shenanigans of [`own_ref!`] ⚠️
///
/// ### Example
///
/// ```rust
/// use ::own_ref::prelude::*;
///
/// fn unsize_to_slice(
///     o: OwnRef<'_, [u8; 42]>,
/// ) -> OwnRef<'_, [u8]>
/// {
///     ::own_ref::unsize!(o)
/// }
///
/// fn unsize_to_trait(
///     o: OwnRef<'_, [u8; 42]>,
/// ) -> OwnRef<'_, dyn ::core::fmt::Debug>
/// {
///     ::own_ref::unsize!(o)
/// }
/// ```
///
/// [Unsize]: ::core::marker::Unsize
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
