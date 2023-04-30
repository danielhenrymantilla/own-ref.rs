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
}
