//! Module and APIs to combine [`OwnRef`]s with [`Pin`]ning.
//!
//! Granted, at first glance, not only is the notion intellectually pleasing,
//! but it also makes sense to combine these two abstractions, conceptually speaking
//! (a `pinned_own_ref!(f)` being expected to behave as a more powerful
//! <code>[pin!]\(f\)</code>, with some of the ownership semantics of
//! <code>[Box::pin]\(f)</code> sprinkled on top of it).
//!
//! Alas,
//!
//!   - if you have a proper mental model of how <code>[OwnRef]\<\'slot, T\></code>
//!     is "just" a "glorified" [`Drop`]-imbued wrapper around
//!     <code>\&\'slot mut [ManuallyDrop]\<T\></code>,
//!     with no control over the backing storage used for that `T` whatsoever;
//!
//!       - (especially around how it may be reclaimed and reüsed)
//!
//! [ManuallyDrop]: ::core::mem::ManuallyDrop
//!
//!   - and if you are also aware of how important the [`Drop` guarantee of
//!     `Pin`] is;
//!
//! [`Drop` guarantee of `Pin`]: https://doc.rust-lang.org/1.78.0/std/pin/index.html#subtle-details-and-the-drop-guarantee
//!
//! then it should be quite puzzling, surprising, and/or unexpected for
//! [`OwnRef`] and [`Pin`] to ever get to be remotely compatible.
//!
//! Let's illustrate the issue at which I am hinting:
//!
//!  1. ### The [`Drop` guarantee of `Pin`] in a nutshell, illustrated by a silly API
//!
//!     ```rust
//!     use ::std::{
//!         pin::Pin,
//!         ptr,
//!         sync::atomic::{AtomicBool, Ordering},
//!         thread,
//!         time::Duration,
//!     };
//!
//!     #[derive(Default)]
//!     pub struct Example {
//!         pending: AtomicBool,
//!         /// `impl !Unpin for Self {}`, sort to speak.
//!         _pin_sensitive: ::core::marker::PhantomPinned,
//!     }
//!
//!     impl Drop for Example {
//!         fn drop(&mut self) {
//!             while self.pending.load(Ordering::Acquire) {
//!                 // spin-looping is generally AWFUL, performance-wise.
//!                 // But that question is besides the point / irrelevant
//!                 // for this example.
//!             }
//!         }
//!     }
//!
//!     impl Example {
//!         fn spawn_task<'not_necessarily_static>(
//!             self: Pin<&'not_necessarily_static Self>,
//!         )
//!         {
//!             // Check and ensure we're the only one being spawned.
//!             assert_eq!(false, self.pending
//!                                   .swap(true, Ordering::AcqRel)
//!             );
//!             // Get `&self.pending`, but with the lifetime erased.
//!             let ptr = UnsafeAssumeSend(ptr::NonNull::from(&self.pending));
//!             thread::spawn(move || {
//!                 thread::sleep(Duration::from_secs(10));
//!                 let at_pending: &AtomicBool = unsafe {
//!                     // SAFETY?
//!                     // Yes! Thanks to the `Drop` guarantee of `Pin`.
//!                     // Since `*self` is not `Unpin`, and since `*self` has
//!                     // been *witnessed*, even if just for an instant,
//!                     // behind a `Pin`-wrapped pointer,
//!                     // then the `Pin` contract gurantees to *us* witnesses
//!                     // that the `*self` memory shall not be invalidated
//!                     // (moved elsewhere and/or deällocated) before the
//!                     // drop glue of `*self` has been invoked.
//!                     //
//!                     // So now we know that the `while {}` busy-looping of
//!                     // the drop glue shall be running to prevent this
//!                     // pointer from ever dangling until we set the flag.
//!                     { ptr }.0.as_ref()
//!                 };
//!                 at_pending.store(true, Ordering::Release);
//!             });
//!         }
//!     }
//!
//!     /// Helper to pass raw pointers through a `thread::spawn()` boundary.
//!     struct UnsafeAssumeSend<T>(T);
//!     unsafe impl<T> Send for UnsafeAssumeSend<T> {}
//!     ```
//!
//!     As explained in the `// SAFETY??` comments, this API is sound, no matter
//!     how evil or devious our caller is, since _they_ have the burden of
//!     abiding by the [`Drop` guarantee of `Pin`]. In other words, if _they_
//!     mess up that part around `Drop`, and the call to `.spawn_task()` ends up
//!     resulting in, say, a use-after-free (UAF), because, say, our `Example`
//!     instance is freed/destroyed without running `Example`'s [`drop()`]
//!     glue, then the blame for the resulting Undefined Behavior is on
//!     _them_, not us.
//!
//!  1. ### Violating it with <code>[OwnRef]\<\'\_, T></code> and [`Pin::new_unchecked()`]
//!
//!     ```rust ,no_run
//!     # #[derive(Default)] struct Example(::core::marker::PhantomPinned);
//!     # impl Example { fn spawn_task(self: Pin<&Self>) {} }
//!     #
//!     use ::own_ref::{prelude::*, Slot};
//!
//!     {
//!         let example_backing_memory = &mut slot();
//!         let own_ref_to_example: OwnRef<'_, Example> =
//!             example_backing_memory
//!                 .holding(Example::default())
//!         ;
//!         let pinned_own_ref: Pin<OwnRef<'_, Example>> = unsafe {
//!             // Safety??
//!             // None whatsoever! This is unsound 😬
//!             Pin::new_unchecked(own_ref_to_example)
//!         };
//!         let pinned_shared_ref: Pin<&'_ Example> = pinned_own_ref.as_ref();
//!         // Schedule background thread to access `Example` in 10 seconds.
//!         pinned_shared_ref.spawn_task();
//!
//!         ::core::mem::forget(pinned_own_ref); // disable `pinned_own_ref`'s drop glue.
//!     } // <- the `*example_backing_memory` is deällocated/repurposed, with
//!       //    no wait/busy-looping whatsoever, since there is nothing left to
//!       //    be running the `drop()` glue of `Example` 😬
//!     // 10s-ish later:
//!     /* UAF, and thus, UB */
//!     ::std::thread::sleep(::std::time::Duration::from_secs(11));
//!     ```
//!
//!     <details open><summary>Click to hide the explanation and compare the snippets</summary>
//!
//!     The problem with this API stems from the "zero-cost"-ness of the
//!     <code>[slot()].[holding(…)][Slot::holding()]</code><br/>
//!     <code>[OwnRef]\<\'slot, …\></code>-yielding pattern.
//!
//!     Indeed, the design/ideä of this API is for [`Slot`] to be _inert_,
//!     w.r.t. the `T` it _may_ contain. It will, itself, never try to access or
//!     use it, it's just a bag of bytes which "somebody" else may use at their
//!     own (`&mut`-exclusive) leisure (again, while this talks mostly about
//!     _local_ (stack) storage, the similarity with the
//!     [`alloc()`][::std::alloc::alloc] APIs, in the case of `Box<T>`, should
//!     be quite visible).
//!
//!       - To speak in more concrete implementation-detail-exposing terms, a
//!         <code>[Slot\<T\>][Slot]</code> is just a
//!         <code>[ManuallyDrop]\<T\></code> wearing a fancy _negligee_.
//!
//!         So, much like <code>[ManuallyDrop]\<T\></code>, it is itself
//!         completely unaware and oblivious of whether there is an actually
//!         initialized or active `T` instance in it, so the whole thing is just
//!         ignored, and it itself acts simply as a sheer bag of bytes.
//!
//!     **All of the `T`-interacting logic, _including the [`drop()`] glue_,
//!     thus lies within the resulting <code>[OwnRef]\<\'slot, T\></code>
//!     "handle"**.
//!
//!     Thus, if it gets [forgot][::core::mem::forget]ten, there is nothing
//!     responsible for dropping the `T` memory! This is usually fine (by the
//!     principle of "leak amplification"), but in the case of the
//!     [`Drop` guarantee of `Pin`], it is not acceptable, which makes usage of
//!     [`Pin::new_unchecked()`] on such a pointer unsound.
//!
//!     </details>
//!
//!  1. ### How this module works around the problem
//!
//!     <details open><summary>Click to hide the explanation and compare the snippets</summary>
//!
//!     What if we made our `T`-holding memory a bit smarter then? Right now it
//!     just lends its bytes to whomever asks for them, _naïvely_ expecting the
//!     `T`s inserted therein to be properly taken care of, _naïvely_ trusting
//!     the [`OwnRef`]. But, as we've seen, since the [`OwnRef`] itself may be
//!     [forgot][::core::mem::forget]ten, such naïve/unconditional trust may be
//!     ill-suited: we need more skeptical/distrustful/apprehensive/circumspect
//!     memory: <code>[ManualOption]\<T\></code>!
//!
//!     <img
//!         alt="fry sus meme"
//!         title="fry sus meme"
//!         src="https://gist.github.com/assets/9920355/99afe5d8-3c39-4bd4-9e22-a562da7b53b4"
//!         height="200px"
//!     />
//!
//!     <code>[ManualOption]\<T\></code> is, modulo implementation details,
//!     functionally equivalent to an <code>[Option]\<T\></code>: it may hold a
//!     `T` value inside of it, **and it keeps a runtime flag/discriminant to
//!     know if such a value is there!**
//!
//!     We can then define a special <code>[Pin]\<\&mut [Some][ManualOption::Some]\(T\)\></code>
//!     "auto-[`.unwrap()`][Option::unwrap]ping" handle, which, on [`Drop`],
//!     _clears_ the `Option<T>` referee by [`.set`][Pin::set]ting it back to
//!     [`None`], thereby [`drop_in_place()`][::core::ptr::drop_in_place]-ing
//!     the `T` value (in the happy / non-[forgot][::core::mem::forget]ten case).
//!
//!     Such a wrapper is a _new_ / **distinct** [`OwnRef`]-like type:
//!
//!     > <code>[OwnRef]\<\'slot, T, [DropFlags::Yes]\></code>
//!
//!       - Notice how a normal [`OwnRef`], _by default_, is actually an
//!         <code>[OwnRef]\<\'slot, T, [DropFlags::No]\></code>.
//!
//!     And, in the sad/silly [forgot][::core::mem::forget]ten case, we still
//!     have the [`drop()`] glue of our <code>[ManualOption]\<T\></code> backing
//!     memory holder running, which much like for an <code>[Option]\<T\></code>,
//!     **runtime-checks** whether there is indeed a [`ManualOption::None`]
//!     inside of it (_detecting whether proper disposal of its value has been
//!     done_), **else it [`drop_in_place()`][::core::ptr::drop_in_place]s the
//!     `T` value lying therein _by itself_!**
//!
//!       - For those wondering, the [`ManualOption`] itself cannot be
//!         forgotten, since it only lends itself to
//!         [`holding()`][ManualOption::holding] a value of type `T` through
//!         a <code>**[Pin]**\<\&mut [Self][ManualOption]\></code> reference,
//!         and it is itself <code>\![Unpin]</code>, which means we are now
//!         ourselves meeting all the criteria to benefit from the
//!         [`Drop` guarantee of `Pin`] 🤯
//!
//!       - Notice how, at the end of the day, the only role played by this
//!         <code>[Some][ManualOption::Some]/[None][ManualOption::None]</code>
//!         discriminant/flag is for _dropping_ purposes.
//!
//!         It thus plays a role very similar to the language built-in
//!         _drop flags_ of Rust:
//!
//!         ```rust
//!         # let some_condition = true;
//!         {
//!             let s;
//!          // let mut is_some = false; // <- "drop flag".
//!             if some_condition {
//!                 s = String::from("to be freed");
//!              // is_some = true;
//!             }
//!         } // <- frees the `String` iff `s` `is_some`.
//!         ```
//!
//!         being equivalent to:
//!
//!         ```rust
//!         # let some_condition = true;
//!         {
//!             let mut s = None;
//!             if some_condition {
//!                 s = Some(String::from("to be freed"));
//!             }
//!         } // <- frees the `String` iff `s` `.is_some()`.
//!         ```
//!
//!         Hence why the combined usage of a [`ManualOption`] with an
//!         <code>[OwnRef]\<\'slot, T, [DropFlags::Yes]\></code>
//!         is said to be using _drop flags_.
//!
//!     All this results in a sound, and non-`unsafe`, API 😇:
//!
//!     </details>
//!
//!     ```rust
//!     # #[derive(Default)] struct Example(::core::marker::PhantomPinned);
//!     # impl Example { fn spawn_task(self: Pin<&Self>) {} }
//!     #
//!     use ::own_ref::{prelude::*, pin::ManualOption};
//!
//!     {
//!         let example_backing_memory = pin!(pin::slot());
//!         //                       or `pin::slot!()` shorthand.
//!         let pinned_own_ref: Pin<OwnRef<'_, Example, _>> =
//!             example_backing_memory
//!                 .holding(Example::default())
//!         ;
//!         let pinned_shared_ref: Pin<&'_ Example> = pinned_own_ref.as_ref();
//!         // Schedule background thread to access `Example` in 10 seconds.
//!         pinned_shared_ref.spawn_task();
//!         ::core::mem::forget(pinned_own_ref); // disable `pinned_own_ref`'s drop glue.
//!     } // <- the `*example_backing_memory` is …
//!       //                                     actually detecting the above `forget()`
//!       //                                     and taking `Drop` matters into its own hands,
//!       //                                     running `Example`'s drop glue,
//!       //                                     preventing the unsoundness! 🥳🥳💪
//!     /* spin-looping until the spawned thread is done with `Example`. */
//!     ```
//!

use super::*;
use ::core::marker::PhantomPinned;

/// Even though, structurally, we could have had this impl without writing it
/// (by virtue of not using [`PhantomPinned`]), I personally find that to be
/// too terse, and brittle.
///
/// We *really* want `T : !Unpin` to make `ManualOption<T> : !Unpin`!
impl<T : Unpin> Unpin for ManualOption<T> {}

/// Moral equivalent of an <code>[Option]\<T\></code>, modulo discriminant
/// layout implementation details (which are currently not exposed as part of
/// the API, but if there is a desire for it, it could be).
#[repr(C)]
pub
struct ManualOption<T> {
    pub(in crate)
    is_some: bool,

    pub(in crate)
    value: MU<T>,

    /// default `!Unpin` when the `impl<T: Unpin> Unpin` above does not apply.
    _pin_sensitive: PhantomPinned,
}

impl<T> Drop for ManualOption<T> {
    #[inline]
    fn drop(&mut self)
    {
        if ::core::mem::needs_drop::<T>() && self.is_some {
            unsafe {
                self.value.as_mut_ptr().drop_in_place()
            }
        }
    }
}

impl<T> From<Option<T>> for ManualOption<T> {
    fn from(o: Option<T>)
      -> ManualOption<T>
    {
        match o {
            Some(v) => Self::Some(v),
            None => Self::None,
        }
    }
}

impl<T> From<ManualOption<T>> for Option<T> {
    fn from(o: ManualOption<T>)
      -> Option<T>
    {
        let o = MD::new(o);
        o.is_some.then(|| unsafe { o.value.as_ptr().read() })
    }
}

impl<T> ManualOption<T> {
    /// Moral equivalent of [`Option::Some()`][Option::Some].
    #[allow(nonstandard_style)]
    pub
    const
    fn Some(value: T)
      -> Self
    {
        Self {
            is_some: true,
            value: MU::new(value),
            _pin_sensitive: PhantomPinned,
        }
    }

    /// Moral equivalent of [`Option::None`].
    #[allow(nonstandard_style)]
    pub
    const None: Self = Self {
        is_some: false,
        value: MU::uninit(),
        _pin_sensitive: PhantomPinned,
    };

    /// Moral equivalent of [`Option::as_ref()`].
    pub
    fn as_ref(&self)
      -> Option<&T>
    {
        self.is_some.then(|| unsafe {
            self.value.assume_init_ref()
        })
    }

    /// Moral equivalent of [`Option::as_mut()`].
    pub
    fn as_mut(&mut self)
      -> Option<&mut T>
    {
        self.is_some.then(|| unsafe {
            self.value.assume_init_mut()
        })
    }

    /// Same as [`Slot::holding()`], but for it returning a `Pin`ned `value`.
    ///
    /// Uses [runtime drop flags][self] to guard against improper memory leakage,
    /// lest unsoundness ensue.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ::own_ref::prelude::*;
    ///
    /// # let some_condition = true;
    /// let future = async {
    ///     // …
    /// };
    /// let slot = pin::slot!();
    /// let mut future = slot.holding(future);
    /// // Same usability as `pin!(future)`:
    /// let _: Pin<&mut dyn Future<Output = ()>> = future.as_mut();
    /// if some_condition {
    ///     // New capability of `pin::slot!().holding()` vs. `pin!`: early dropping!
    ///     // (much like for `Box::pin`).
    ///     drop(future);
    /// }
    pub
    fn holding<'slot>(
        mut self: Pin<&'slot mut ManualOption<T>>,
        value: T,
    ) -> Pin<OwnRef<'slot, T, DropFlags::Yes>>
    {
        self.set(Self::None);
        unsafe {
            let this = self.get_unchecked_mut();
            this.value.write(value);
            this.is_some = true;
            // We need this cast to a raw pointer because otherwise
            // `addr_of_mut!` shrinks provenance…
            // Biggest footgun in Rust, imho.
            let this: *mut Self = this;
            // `OwnRef<'_, T, DropFlags::Yes>`' drop glue relies on this.
            #[cfg(feature = "offset_of")] {
                impl<T> ManualOption<T> {
                    const FIELD_OFFSET_ASSERTION: () = assert!(
                        (
                            ::core::mem::offset_of!(Self ,value)
                            -
                            ::core::mem::align_of::<T>()
                        ) == (
                            ::core::mem::offset_of!(Self ,is_some)
                        )
                    );
                }
                () = ManualOption::<T>::FIELD_OFFSET_ASSERTION;
            }
            // Safety:
            //   - we have just `const`-checked the layout assumption.
            //   - our raw pointer does indeed behave similarly to a `&mut MD<T>`,
            //     insofar if the `OwnRef` is indeed dropped, then the `is_some`
            //     flag is cleared so that our `ManualOption<T>` do nothing,
            //     thence acting like a `ManuallyDrop<T>`.
            let own_ref = OwnRef::from_raw(
                // We have made sure to keep provenance over all of `*self`,
                // so that the resulting pointer be still allowed to,
                // eventually, mutate back the `.is_some` field.
                ::core::ptr::addr_of_mut!((*this).value).cast(),
                [],
            );
            // Safety:
            //   - The `Deref{,Mut}` impls are not silly.
            //   - The value is to be dropped before its backing allocation
            //     (_i.e._, `*self`), is reclaimed/reüsed/rewritten, since, by
            //     the time `ManualOption::drop` runs:
            //       - either `OwnRef` has properly dropped the value (and told
            //          us so by setting `is_some` to `false`);
            //       - or `is_some` is `true`, and we do drop it.
            //     We know this drop/check will run since we have, our`self`es,
            //     been `Pin`ned, and we're never `Unpin` unless the `value` is.
            Pin::new_unchecked(own_ref)
        }
    }
}

impl<'slot, T> OwnRef<'slot, T, DropFlags::Yes> {
    /// Same as [`OwnRef::with()`], but for the `value` being `Pin`ned.
    ///
    /// Uses [runtime drop flags][self] to guard against improper memory leakage,
    /// lest unsoundness ensue.
    pub
    fn with_pinned<R>(
        value: T,
        scope: impl FnOnce(Pin<OwnRef<'_, T, DropFlags::Yes>>) -> R,
    ) -> R
    {
        let yield_ = scope;
        yield_(pin::slot!().holding(value))
    }
}

#[allow(nonstandard_style)]
pub
mod DropFlags {
    //! Type-level `bool`.
    //!
    //! ```rust
    //! # #[cfg(any())] macro_rules! {
    //! enum DropFlags {
    //!     No,
    //!     Yes,
    //! }
    //! # }
    //! ```
    //!
    //! See the [`pin` module][mod@crate::pin] documentation for more information about this.

    /// `DropFlags::No`, used by default by <code>[OwnRef]\<\'\_, T\></code>
    ///
    /// [OwnRef]: crate::OwnRef
    pub enum No {}

    /// `DropFlags::Yes`, used by the [`pin`][mod@crate::pin]-friendly APIs.
    pub enum Yes {}

    // We don't seal this type-level `enum` for the sake of ergonomics, we'll
    // just `panic!` if other instantiations are attempted.
}

/// [`pin!`]-friendly version of [`crate::slot()`].
///
/// Intended to be immediately [`pin!`]ned. Thence the [`slot!`] shorthand.
pub
const
fn slot<T>() -> ManualOption<T> {
    ManualOption::None
}

#[doc(hidden)]
/// Convenience shorthand for <code>[pin!]\([pin::slot()][slot()])</code>.
///
/// To be used with [`.holding()`][ManualOption::holding].
#[macro_export]
macro_rules! ඞpinned_slot {() => (
    ::core::pin::pin!($crate::pin::slot())
)}
#[doc(inline)]
pub use ඞpinned_slot as slot;
