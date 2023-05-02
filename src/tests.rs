use super::*;

#[test]
fn miri() { _main() }

fn _main()
{
    let new = |i| ::scopeguard::guard((), move |()| _ = dbg!(i));
    {
        let _o = own!(new(42));
    }
    {
        let o = own!(new(27));
        drop(o);
    }
    {
        let mut storage = Slot::VACANT;
        let o = storage.hold(new(0));
        drop(o);
        let o = storage.hold(new(1));
        drop(o);
    }
    {
	    let o = own!(String::from("..."));
	    drop(o);
	}
	{
    	let _o: OwnRef<'_, dyn FnOnce()> = own!(|| ());
    }
    {
	    let (storage, storage2) = &mut slots();
	    if false { storage2.hold(()); }
	    let _o: OwnRef<'_, dyn FnOnce()> = unsize!(storage.hold(|| ()));
	}
    {
        let local: &str = &String::from("…");
        let a: OwnRef<'_, &'static str> = own!("");
        let b: OwnRef<'_, &'_ str> = own!(local);
        fn same_lifetime<T>(_: T, _: T) {}
        same_lifetime(a, b);
	}
}

#[apply(compile_fail!)]
fn moves_value_in()
{
    let not_copy = String::from("…");
    ::core::mem::forget(own!(not_copy));
    drop(not_copy); // Error: use of moved value.
}

#[apply(compile_fail!)]
fn not_static()
{
    // Error: temporary value dropped while borrowed
    // (type annotation requires that the borrow be `'static`).
    let _: OwnRef<'static, _> = own!(String::from("…"));
}

#[apply(compile_fail!)]
fn bound_to_scope_of_creation()
{
    let _o = {
        let o: OwnRef<'_, _> = own!(String::from("…"));
        o // Error: temporary is freed at the end of this statement.
    };
}

#[apply(compile_fail!)]
fn lifetime_extension_is_brittle()
{
    use ::core::convert::identity;
    {
        let _o: OwnRef<'_, _> = identity(own!(String::from("…")));
    } // Error: borrow might be used here, when `_o` is dropped
      // note: consider using a `let` binding to create a longer lived value
}
