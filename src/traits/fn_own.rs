//! Quintessential example of a `fn(self)` `dyn`-safe trait.

use crate::OwnRef;

pub trait Sealed<Args> {}

/// Same as [`FnOnce`], but for having been designed with [`OwnRef`]s in mind.
//
// (This is what [`FnOnce`]'s true signature should have been.)
///
/// Allows a:
///
/// > <code>[OwnRef]\<\'\_, dyn \'\_ + Send… + Sync… + [`FnOwn`]\<(i32, u8), Ret = String\></code>
///
/// to be as usable as a:
///
/// > `   Box<    dyn '_ + Send… + Sync… + FnOnce(i32, u8)     -> String>`.
///
/// ## Example
///
/// ```rust
/// use ::own_ref::prelude::*;
///
/// let check_inputs = matches!(::std::env::var("CHECK_INPUTS").as_deref(), Ok("1"));
///
/// let s = String::from("not copy");
/// let f: OwnRef<'_, dyn FnOwn<(i32, u8), Ret = String>> = if check_inputs {
///     own_ref!(|x: i32, y: u8| {
///         assert_eq!(x, 42);
///         assert_eq!(y, 27);
///         /* return */ s
///     })
/// } else {
///     own_ref!(|_, _| s)
/// };
/// let s: String = f.call_ownref_2(42, 27);
/// assert_eq!(s, "not copy");
/// ```
pub
trait FnOwn<Args> : FnOwnRet<Args> + ඞ<Args> {
    /// The moral equivalent of `extern "rust-call" fn call_once`.
    fn call_ownref(self, args: Args)
      -> Self::Ret
    where
        Self : Sized,
    ;

    crate::arities::feed_all!(=> call_ownref_N!);
}

// where
macro_rules! call_ownref_N {(
    $( $N:ident $($I:ident)* )?
) => (
    $(
        ::paste::paste! {
            /// Convenience sugar around [`Self::call_ownref()`].
            ///
            /// These go up to an arity of
            #[doc = crate::arities::max!()]
            /// arguments (some may have been hidden from the docs to keep them
            /// legible).
            fn [< call_ownref$N >]<$($I),*>(
                self,
                $($I: $I),*
            ) -> Self::Ret
            where
                Self : Sized,
                ($($I, )*) : Is<ItSelf = Args>,
            {
                self.call_ownref(Is::cast(($($I, )*)))
            }
        }
    )?
)} use call_ownref_N;

/// The `type Ret = …;` part of [`FnOwn`].
pub
trait FnOwnRet<Args> : Sealed<Args> {
    type Ret;
}

#[doc(hidden)] /** Not part of the public API! */ pub
trait FnOwnUnchecked<Args> : FnOwnRet<Args> {
    // SAFETY(pub): NONE!
    // SAFETY(in crate): it must be sound for the callee to
    // transmute the `&'_ mut Self` to a `OwnRef<'_, Self>`.
    // This is true for `DropFlags::No`, for instance, when `Self`
    // has been `ManuallyDrop<>`-wrapped before being `&mut`-borrowed.
    #[doc(hidden)] /** Not part of the public API! */
    unsafe
    fn ඞdyn_call_ownref(&mut self, _: Args)
      -> Self::Ret
    ;
}
use FnOwnUnchecked as ඞ;

impl<F : FnOwn<Args>, Args> FnOwnUnchecked<Args> for F {
    unsafe
    fn ඞdyn_call_ownref(&mut self, args: Args)
      -> Self::Ret
    {
        unsafe {
            // SAFETY: this being sound is the safety precondition.
            // `<*mut Self>::read()` here is the moral equivalent of `ManuallyDrop::take()`.
            // i.e., `OwnRef::into_inner()`.
            <*mut Self>::read(self)
        }
        .call_ownref(args)
    }
}

pub
trait Is : Sized {
    type ItSelf;
    fn cast(it: Self) -> Self::ItSelf;
}

impl<T> Is for T {
    type ItSelf = Self;
    #[inline(always)]
    fn cast(it: Self) -> Self::ItSelf { it }
}

crate::arities::feed_all!(=> impls!);
// where
macro_rules! impls {
    (
        $($I:ident)*
    ) => (
        impl<F, Ret $(, $I)*>
            Sealed<($($I, )*)>
        for
            F
        where
            F : FnOnce($($I),*) -> Ret,
        {}

        impl<F, Ret $(, $I)*>
            FnOwnRet<($($I, )*)>
        for
            F
        where
            F : FnOnce($($I),*) -> Ret,
        {
            type Ret = Ret;
        }

        impl<F, Ret $(, $I)*>
            FnOwn<($($I, )*)>
        for
            F
        where
            F : FnOnce($($I),*) -> Ret,
        {
            fn call_ownref(self, ($($I, )*): ($($I, )*))
              -> Self::Ret
            {
                self($($I),*)
            }
        }
    );
} use impls;

impl<'slot, Args, F : ?Sized + FnOwn<Args>>
    Sealed<Args>
for
    OwnRef<'slot, F>
{}

impl<'slot, Args, F : ?Sized + FnOwn<Args>>
    FnOwnRet<Args>
for
    OwnRef<'slot, F>
{
    type Ret = F::Ret;
}

impl<'slot, Args, F : ?Sized + FnOwn<Args>>
    FnOwn<Args>
for
    OwnRef<'slot, F>
{
    fn call_ownref(self, args: Args)
      -> Self::Ret
    {
        unsafe {
            // SAFETY: we are indeed `ManuallyDrop`-wrapping beforehand.
            F::ඞdyn_call_ownref(
                &mut *::core::mem::ManuallyDrop::new(self),
                args,
            )
        }
    }
}
