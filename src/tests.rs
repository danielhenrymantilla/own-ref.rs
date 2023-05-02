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
    }
    {
        let (storage, storage2) = &mut slots();
        if false { storage2.holding(()); }
        let _o: OwnRef<'_, dyn FnOnce()> = unsize!(storage.holding(|| ()));
    }
    {
        let local: &str = &String::from("…");
        let a: OwnRef<'_, &'static str> = own_ref!("");
        let b: OwnRef<'_, &'_ str> = own_ref!(local);
        fn same_lifetime<T>(_: T, _: T) {}
        same_lifetime(a, b);
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
