use super::*;

#[test]
fn main()
{
    let new = |i| ::scopeguard::guard((), move |()| _ = dbg!(i));
    {
        let _o = own_ref!(new(42));
    }
    {
        let o = own_ref!(new(27));
        drop(o);
    }
    {
        let mut storage = Slot::VACANT;
        let o = storage.holding(new(0));
        drop(o);
        let o = storage.holding(new(1));
        drop(o);
    }
    {
        let o = own_ref!(String::from("..."));
        drop(o);
    }
    {
        let _o: OwnRef<'_, dyn FnOnce()> = own_ref!(|| ());
        // Alas, not much we can do with an `OwnRef<…dyn FnOnce()>`.
    }
    {
        let s = String::from("not copy");
        let o: OwnRef<'_, dyn Send + Unpin + crate::FnOwn<(), Ret = String>> = own_ref!(|| s);
        let s: String = o.call_ownref_0();
        assert_eq!(s, "not copy");
    }
    {
        let (storage, storage2, storage3) = &mut slots();
        if false {
            storage2.holding(());
        }
        if true {
            storage3.holding(42);
        }
        let _o: OwnRef<'_, dyn FnOnce()> = ::own_ref::unsize!(storage.holding(|| ()));
    }
}


/// RIP 😭
#[cfg(doctest)]
#[apply(compile_fail!)]
fn alas_non_covariant()
{
    let local: &str = &String::from("…");
    let a: OwnRef<'_, &'static str> = own_ref!("");
    let b: OwnRef<'_, &'_ str> = own_ref!(local);
    fn same_lifetime<T>(_: T, _: T) {}
    same_lifetime(a, b);
}

#[test]
fn branches() {
    let it: OwnRef<'_, [String]> = if true {
        own_ref!([String::from("one")])
    } else {
        own_ref!([String::from("two"), String::from("three")])
    };
    drop(it);
}

#[test]
fn hrtb() {
    let not_copy = String::new();
    let f = |_: &str| {
        drop(not_copy);
    };
    let f = own_ref!(f);
    if true {
        let f: OwnRef<'_, dyn for<'any> FnOwn<(&'any str, ), Ret = ()>> = unsize!(f);
        {
            let local = String::from("local");
            FnOwn::call_ownref_1(f, &local[..]);
        }
    } else {
        let local = String::from("local");
        FnOwn::call_ownref_1(f, &local[..]);
    }
}

#[cfg(doctest)]
#[apply(compile_fail!)]
fn moves_value_in()
{
    let not_copy = String::from("…");
    ::core::mem::forget(own_ref!(not_copy));
    drop(not_copy); // Error: use of moved value.
}

#[cfg(doctest)]
#[apply(compile_fail!)]
fn not_static()
{
    // Error: temporary value dropped while borrowed
    // (type annotation requires that the borrow be `'static`).
    let _: OwnRef<'static, _> = own_ref!(String::from("…"));
}

#[cfg(doctest)]
#[apply(compile_fail!)]
fn bound_to_scope_of_creation()
{
    let _o = {
        let o: OwnRef<'_, _> = own_ref!(String::from("…"));
        o // Error: temporary is freed at the end of this statement.
    };
}

#[cfg(doctest)]
#[apply(compile_fail!)]
fn lifetime_extension_is_brittle()
{
    use ::core::convert::identity;
    {
        let _o: OwnRef<'_, _> = identity(own_ref!(String::from("…")));
    } // Error: borrow might be used here, when `_o` is dropped
      // note: consider using a `let` binding to create a longer lived value
}

#[test]
fn robust_way()
{
    use ::core::convert::identity;
    let storage = &mut slot();
    {
        let _o: OwnRef<'_, _> = identity(storage.holding(String::from("…")));
    }
}

/// For those unconvinced of the need to be non-covariant over `T` in the
/// `DropFlags` case, replace this with `#[test]`, and the
/// `_non_covariant_in_case_of_drop_flags` field, with a `PD<fn(&())>` (so that
/// it becomes covariant again). Then, witness the might of
/// `cargo +nightly miri test`.
#[cfg(doctest)]
#[apply(compile_fail!)]
fn guard_against_covariance_if_drop_flags() {
    let storage = pinned_slot!();
    struct PrintOnDrop<'r>(&'r str);
    impl Drop for PrintOnDrop<'_> {
        fn drop(&mut self) {
            dbg!(self.0);
        }
    }
    let o = storage.holding(PrintOnDrop("static"));
    {
        let local = String::from("local");
        let mut o = o; // needs covariance!
        o.0 = &local[..]; // for this assignment to compile.

        ::core::mem::forget(o); // if evil/careless.
    }
    /* implicit `drop(storage)`, which in turn drops the `PrintOnDrop`. */
}
