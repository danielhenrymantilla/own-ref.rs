use ::own_ref::prelude::*;

#[dyn_safe_owned_dispatch(
    as pub trait DynFooExt,
    owned_dispatch_naming_template = "dyn_{}",
)]
pub trait Foo<T> {
    fn foo(self, _: i8, a: bool)
      -> bool
    ;

    fn bar(&self, _: i8)
      -> bool
    {
        true
    }

    fn baz<U, const C: bool>(self)
    where
        Self : Sized,
    {}
}

impl Foo<()> for OwnRef<'_, dyn Foo<()>> {
    fn foo(self, it: i8, b: bool) -> bool {
        // Available _via_ the `OwnRefFoo` extension trait created above.
        self.dyn_foo(it, b)
    }
}

fn main() {}
