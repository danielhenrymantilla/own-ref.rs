//! <code>[OwnRef]\<\'\_, dyn Send… + Sync… + [Any]\>::[downcast][OwnRef::downcast]</code>

use ::core::any::{Any, TypeId};
use crate::OwnRef;

    impl<'slot, T : ?Sized> OwnRef<'slot, T> {
        /// The moral equivalent of [`Box::downcast`], but for [`OwnRef`]s.
        ///
        /// > More like `.owncast()`, am I right? 🥁
        ///
        /// ## Example
        ///
        /// ```rust
        /// #![forbid(unsafe_code)]
        ///
        /// use ::core::any::{Any, TypeId};
        /// use ::own_ref::prelude::*;
        ///
        /// fn too_generic<T : 'static>(it: T) {
        ///     // Say we want to do something special if `T` is a `String`.
        ///
        ///     match own_ref!(: T = it).downcast::<String>() {
        ///         // Ok, `T = String` here, and this property is embodied
        ///         // by `s: &own String` in this branch:
        ///         Ok(own_s) => {
        ///             let s: String = own_s.deref_move();
        ///             // …
        ///         },
        ///         Err(own_t) => {
        ///             let it: T = own_t.deref_move();
        ///         },
        ///     }
        /// }
        /// ```
        pub
        fn downcast<U>(
            self: OwnRef<'slot, T>,
        ) -> Result<
                OwnRef<'slot, U>,
                OwnRef<'slot, T>,
            >
        where
            T : Any,
            U : Any,
        {
            let _checked_eq @ true = (&*self).type_id() == TypeId::of::<U>()
            else {
                return Err(self);
            };
            let (ptr, lt) = OwnRef::into_raw(self);
            Ok(unsafe {
                // Safety: same layout of thin pointers,
                // and `TypeId`s have just been checked for equality.
                OwnRef::from_raw(ptr.cast::<U>(), lt)
            })
        }
    }
