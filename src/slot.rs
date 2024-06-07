use super::*;

/// Direct & explicit [`OwnRef`] backing memory/storage management.
///
/// Reserves local memory/storage/a _slot_ which shall be capable of
/// [`.holding()`][`Slot::holding()`] an owned value of type `T` in order to
/// expose an [`OwnRef<'slot, T>`] without the need for macros nor callbacks.
///
///   - The very design and point of [`OwnRef`] is to split the _conceptual
///     ownership_ (_i.e._, `drop` ability and responsibility) of some `value: T`
///     from the memory management of the _backing storage_
///     [holding][`Slot::holding()`] the bytes that constitute this value.
///     Mainly, it is perfectly fine to _consume_, in an owning fashion (_e.g._,
///     by `drop`ping it), the `T` itself, whilst this backing memory outlives it /
///     with the backing memory oblivious to this fact / none the wiser.
///
/// And yet, the <code>let r = [own_ref!]\(value\);</code> expression only yields
/// the "handle" owning the `value`, with no such backing storage in sight.
///
/// This is achieved thanks to the expression _temporaries_, and very convoluted
/// hoops to get Rust to lengthen the lifetime of these as much as possible.
///
/// But there are limits to what these hoops can do. The following, for
/// instance, fails to compile:
///
/// ```rust ,compile_fail
/// use ::own_ref::prelude::*;
///
/// let some_option: Option<String> = // ...
/// # None;
///
/// let some_ownref: Option<OwnRef<'_, String>> =
///     some_option.map(|s| own_ref!(s))
/// ;
/// dbg!(&some_ownref);
/// ```
///
///   - <details><summary>Click here to see the error message</summary>
///
///     ```rust ,ignore
///     # r#"
///     error[E0515]: cannot return value referencing temporary value
///       --> src/slot.rs:34:25
///        |
///     11 |     some_option.map(|s| own_ref!(s))
///        |                         ^^^^^^^^^^^
///        |                         |
///        |                         returns a value referencing data owned by the current function
///        |                         temporary value created here
///     # "#
///     ```
///
///     </details>
///
///
/// This is when [`slot()`] shines: by explicitly and directly managing the
/// life-span of this backing storage, we get to lengthen the (maximum) `'slot`
/// lifetime of our [`OwnRef<'slot, T>`], resulting in an [`OwnRef`]
/// which can be used for longer.
///
/// ```rust
/// use ::own_ref::prelude::*;
///
/// let some_option: Option<String> = // ...
/// # None;
///
/// // 1. Declare a `let storage = &mut slot();` sufficiently early/high in the
/// // `fn` body:
/// let storage = &mut slot(); // 👈
///
/// // 2. Use `storage.holding(value)` instead of `own_ref!(value)`.
/// let some_ownref: Option<OwnRef<'_, String>> =
///  // some_option.map(|s| own_ref!(s))        // ❌
///     some_option.map(|s| storage.holding(s)) // ✅
/// ;
/// // 3. Profit™
/// dbg!(&some_ownref);
/// ```
///
/// [`OwnRef<'slot, T>`]: OwnRef
///
/// The too-astute-for-their-own-good/awarerer reader may suggest that lifetime
/// extension shenanigans could still be applied here:
///
/// ```rust
/// use ::own_ref::prelude::*;
///
/// let some_option: Option<String> = // ...
/// # None;
///
/// let some_ownref: Option<OwnRef<'_, String>> =
///     if let Some(s) = some_option {
///         Some {
///             0: own_ref!(s), // 🤡
///         }
///     } else {
///         None
///     }
/// ;
/// dbg!(&some_ownref);
/// ```
///
/// And whilst indeed a nifty trick, it's not something as flexible as
/// explicit `slot()` lifetime management.
#[inline(always)]
pub
const
fn slot<T>()
  -> Slot<T>
{
    Slot::VACANT
}

/// Convenience function around [`slot()`], to batch-produce _tuples_ of such
/// `slot()`s:
///
/// ```rust
/// use ::own_ref::prelude::*;
///
/// let (a, b, c) = &mut slots();
/// # [a, b, c].iter_mut().for_each(|s| _ = s.holding(()));
/// // is the same as:
/// let a = &mut slot();
/// let b = &mut slot();
/// let c = &mut slot();
/// # [a, b, c].iter_mut().for_each(|s| _ = s.holding(()));
/// ```
///
/// For instance, prior to Rust [1.79.0], the "trailing temporaries" of
/// `if { … } else { … }` and `match { =>` branches, for instance, would not get
/// lifetime-extended to the parent scope.
///
/// [1.79.0]: https://releases.rs/docs/1.79.0/
///
/// This, in turn, used to require multiple [`slots()`] quite often:
///
/// ```rust
/// use ::own_ref::{prelude::*, unsize};
///
/// let (a, b, c) = &mut slots();
/// let f: OwnRef<'_, dyn FnOwn<(), Ret = String>> = match ::std::env::args().len() {
///     0 => unsize!(a.holding(|| "<none>".into())),
///     1 => {
///         let arg: String = ::std::env::args().next().unwrap();
///         unsize!(b.holding(move || {
///             dbg!(&arg);
///             // move out of capture: `FnOnce`! And yet no `Box` nor `Option::unwrap()` needed 💪
///             arg
///         }))
///     },
///     _ => unsize!(c.holding(|| "<too many>".into())),
/// };
/// f.call_ownref_0();
/// ```
///
///   - See [`FnOwn`].
///
/// With Rust ≥ [1.79.0], the previous snippet can be simplified down to:
///
/// [1.79.0]: https://releases.rs/docs/1.79.0/
///
/// ```rust
/// use ::own_ref::{prelude::*, unsize};
///
/// let f: OwnRef<'_, dyn FnOwn<(), Ret = String>> = match ::std::env::args().len() {
///     0 => own_ref!(|| "<none>".into()),
///     1 => {
///         let arg: String = ::std::env::args().next().unwrap();
///         own_ref!(move || {
///             dbg!(&arg);
///             // move out of capture: `FnOnce`! And yet no `Box` nor `Option::unwrap()` needed 💪
///             arg
///         })
///     },
///     _ => own_ref!(|| "<too many>".into()),
/// };
/// f.call_ownref_0();
/// ```
///
#[inline(always)]
pub
const
fn slots<Slots>()
  -> Slots
where
    Slots : TupleSlots,
{
    Slots::TUPLE_SLOTS
}

/// The output of [`slot()`].
pub
struct Slot<T>(
    MU<T>,
);

impl<T> Slot<T> {
    /// Same as [`slot()`]. Or rather, [`slot()`] is the same as [`Slot::VACANT`].
    ///
    /// The issue with the clearly zero-cost [`Slot::VACANT`] expression is that
    /// Rust may lint against `&mut <constant>`, such as with
    /// <code>let slot = &mut [Slot::VACANT];</code>, which it does not do for
    /// <code>let slot = &mut [slot()]</code>.
    pub
    const VACANT: Self = Self(MU::uninit());

    /// Main non-macro, non-scoped, non-`unsafe` constructor for an [`OwnRef`].
    #[inline]
    pub
    fn holding<'slot>(self: &'slot mut Slot<T>, value: T)
      -> OwnRef<'slot, T>
    {
        self.0.holding(value)
    }
}

/// Allows direct usage of `.holding()` on `MaybeUninit<T>` storage.
#[extension(pub trait MaybeUninitExt)]
impl<T> MU<T> {
    #[inline]
    fn holding<'slot>(&'slot mut self, value: T)
      -> OwnRef<'slot, T>
    {
        let r: &'slot mut T = self.write(value);
        unsafe {
            OwnRef::from_raw(r, [])
        }
    }
}

pub
trait TupleSlots {
    const TUPLE_SLOTS: Self;
}

crate::arities::feed_all!(=> impls!);
// where
macro_rules! impls {
    (
        $($I:ident)*
    ) => (
        impl< $($I),* > TupleSlots for ( $(Slot<$I>, )* )
        {
            const TUPLE_SLOTS: Self = ( $(Slot::<$I>::VACANT, )* );
        }
    )
} use impls;
