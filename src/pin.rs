//! Module and APIs to combine [`OwnRef`]s with [`Pin`]ning.
//!
//! Granted, it is intellectually pleasing and, at first glance, conceptually
//! making sense (a `pinned_own_ref!(f)` being expected to behave as a more
//! powerful <code>[pin!]\(f\)</code>, with some of the ownership semantics of
//! <code>[Box::pin]\(f)</code> sprinkled on top of it).
//!
//! Alas,
//!
//!   - if you have a proper mental model of how <code>[OwnRef]\<\'slot, T\></code>
//!     is "just" a "glorified" [`Drop`]-imbued wrapper around
//!     <code>\&\'slot mut [ManuallyDrop]\<T\></code>,
//!     with no control over the backing storage used for that `T` whatsoever
//!
//!       - (especially around how it may be reclaimed and reüsed),
//!
//! [ManuallyDrop]: ::core::mem::ManuallyDrop
//!
//!   - and if you are also aware of how important the [`Drop` guarantee of
//!     `Pin`] is,
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
//!     ```rust
//!     # #[derive(Default)] struct Example(::core::marker::PhantomPinned);
//!     # impl Example { fn spawn_task(self: Pin<&Self>) {} }
//!     #
//!     use ::own_ref::{prelude::*, Slot};
//!
//!     {
//!         let example_backing_memory = &mut Slot::VACANT; // or `slot()` shorthand.
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
//!     ```
//!
//!  1. ### How this module works around the problem
//!
//!     ```rust
//!     # #[derive(Default)] struct Example(::core::marker::PhantomPinned);
//!     # impl Example { fn spawn_task(self: Pin<&Self>) {} }
//!     #
//!     use ::own_ref::{prelude::*, pin::ManualOption};
//!
//!     {
//!         let example_backing_memory = pin!(ManualOption::None); // or `pinned_slot!()` shorthand.
//!         let pinned_own_ref: Pin<OwnRef<'_, Example, _>> =
//!             example_backing_memory
//!                 .holding(Example::default())
//!         ;
//!         let pinned_shared_ref: Pin<&'_ Example> = pinned_own_ref.as_ref();
//!         // Schedule background thread to access `Example` in 10 seconds.
//!         pinned_shared_ref.spawn_task();
//!         ::core::mem::forget(pinned_own_ref); // disable `pinned_own_ref`'s drop glue.
//!     } // <- the `*example_backing_memory` is …
//!       //                                     …
//!       //                                     actually detecting the above `forget()`
//!       //                                     and taking `Drop` matters into its own hands,
//!       //                                     running `Example`'s drop glue,
//!       //                                     preventing the unsoundness! 🥳🥳💪
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
    #[allow(nonstandard_style)]
    pub
    fn Some(value: T)
      -> Self
    {
        Self {
            is_some: true,
            value: MU::new(value),
            _pin_sensitive: PhantomPinned,
        }
    }

    #[allow(nonstandard_style)]
    pub
    const None: Self = Self {
        is_some: false,
        value: MU::uninit(),
        _pin_sensitive: PhantomPinned,
    };

    pub
    fn as_ref(&self)
      -> Option<&T>
    {
        self.is_some.then(|| unsafe {
            self.value.assume_init_ref()
        })
    }

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
    /// Uses runtime drop flags to guard against improper memory leakage, lest unsoundness ensue.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ::own_ref::prelude::*;
    ///
    /// let future = async {
    ///     // …
    /// };
    /// let slot = pinned_slot!();
    /// let mut future = slot.holding(future);
    /// let _: Pin<&mut dyn Future<Output = ()>> = future.as_mut();
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
                            ::core::mem::offset_of!(
                                crate::pin::ManualOption<T> ,value
                            )
                            -
                            ::core::mem::align_of::<T>()
                        )
                        ==
                        ::core::mem::offset_of!(
                            crate::pin::ManualOption<T> ,is_some
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
                // so that the resulting pointer is still allowed to,
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
    /// Uses runtime drop flags to guard against improper memory leakage, lest unsoundness ensue.
    pub
    fn with_pinned<R>(
        value: T,
        scope: impl FnOnce(Pin<OwnRef<'_, T, DropFlags::Yes>>) -> R,
    ) -> R
    {
        let yield_ = scope;
        yield_(pinned_slot!().holding(value))
    }
}

#[allow(nonstandard_style)]
pub
mod DropFlags {
    //! Type-level `bool`
    //!
    //! ```rust
    //! # #[cfg(any())] macro_rules! {
    //! enum DropFlags {
    //!     No,
    //!     Yes,
    //! }
    //! # }
    //! ```

    pub enum No {}
    pub enum Yes {}

    // pub trait Marker : seal::Sealed {}

    // impl Marker for No {}
    // impl Marker for Yes {}

    // mod seal {
    //     pub trait Sealed : 'static + Send + Sync {}
    //     impl Sealed for super::No {}
    //     impl Sealed for super::Yes {}
    // }
}

/// Convenience shorthand for <code>[pin!]\([ManualOption::None])</code>.
///
/// To be used with [`.holding()`][ManualOption::holding].
#[macro_export]
macro_rules! pinned_slot {() => (
    ::core::pin::pin!($crate::pin::ManualOption::None)
)}
pub use pinned_slot;
