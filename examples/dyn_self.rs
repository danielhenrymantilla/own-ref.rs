#[::own_ref_proc_macros::dyn_self]
pub trait Foo<T> {
    fn foo(self, _: i8) -> bool;
    fn bar(&self, _: i8) -> bool;
    // fn baz(_: i8) -> bool where Self : Sized;
    // type Ret;
}

fn demo(it: ::own_ref::OwnRef<'_, dyn Foo<()>>)
{
    it.foo(42);
}

fn main() {}
