//! <code>[OwnRef]\<\'\_, dyn Send… + Sync… + [Any]\>::[downcast][OwnRef::downcast]</code>

use ::core::any::{Any, TypeId};
use crate::OwnRef;

with_send_sync! {}
with_send_sync! { Send + }
with_send_sync! { Sync + }
with_send_sync! { Send + Sync + }
// where
macro_rules! with_send_sync {(
    $($SendSync:tt)*
) => (
    impl<'slot> OwnRef<'slot, dyn $($SendSync)* Any> {
        /// The moral equivalent of [`Box::downcast`], but for [`OwnRef`]s.
        ///
        /// ## Example
        ///
        /// ```rust
        /// use ::core::any::{Any, TypeId};
        /// use ::own_ref::prelude::*;
        ///
        /// fn too_generic<T : 'static>(it: T) {
        ///     #![forbid(unsafe_code)]
        ///     // do something if `T` is a `String`
        ///     if TypeId::of::<T>() == TypeId::of::<String>() {
        ///         // Ok, `T = String` here, and this property is embodied
        ///         // by `s: String` in this branch:
        ///         let s: String =
        ///             OwnRef::<'_, dyn Any>::downcast::<String>(own_ref!(it))
        ///                 .unwrap_or_else(|_| unreachable!())
        ///                 .into_inner()
        ///         ;
        ///         // …
        ///     }
        /// }
        /// ```
        pub
        fn downcast<T: 'static>(
            self: OwnRef<'slot, dyn $($SendSync)* Any>,
        ) -> Result<
                OwnRef<'slot, T>,
                OwnRef<'slot, dyn $($SendSync)* Any>,
            >
        {
            let _checked_eq @ true = (&*self).type_id() == TypeId::of::<T>()
            else {
                return Err(self);
            };
            let (ptr, lt) = OwnRef::into_raw(self);
            Ok(unsafe {
                // Safety: same layout of thin pointers,
                // and `TypeId`s have just been checked for equality.
                OwnRef::from_raw(ptr as *mut ::core::mem::ManuallyDrop<T>, lt)
            })
        }
    }
)} use with_send_sync;
