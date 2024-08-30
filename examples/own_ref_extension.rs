#[::own_ref_proc_macros::own_ref_extension(
    pub trait FooOwn,
    method_rename_logic = "ownref_{}",
)]
pub trait Foo<T> {
    fn foo(self, _: i8) -> bool;
    fn bar(&self, _: i8) -> bool;
    // fn baz(_: i8) -> bool where Self : Sized;
    // type Ret;
}

fn demo(it: ::own_ref::OwnRef<'_, dyn Foo<()>>)
{
    it.ownref_foo(42);
}

fn main() {}
