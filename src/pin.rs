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
        self.is_some.then(|| unsafe { self.value.assume_init_ref() })
    }

    pub
    fn as_mut(&mut self)
      -> Option<&mut T>
    {
        self.is_some.then(|| unsafe { self.value.assume_init_mut() })
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
            let own_ref = OwnRef::from_raw(
                // Make sure to keep provenance over all of `*self`.
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
