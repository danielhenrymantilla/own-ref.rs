use super::*;

pub
const
fn slot<T>()
  -> Slot<T>
{
    Slot::VACANT
}

pub
fn slots<Slots>()
  -> Slots
where
    Slots : TupleSlots,
{
    Slots::tuple_slots()
}

pub
struct Slot<T>(
    MU<T>,
);

impl<T> Slot<T> {
    pub
    const VACANT: Self = Self(MU::uninit());

    pub
    fn holding(self: &'_ mut Slot<T>, value: T)
      -> OwnRef<'_, T>
    {
        self.0.holding(value)
    }
}

#[extension(pub trait MaybeUninitExt)]
impl<T> MU<T> {
    fn holding<'r>(&'r mut self, value: T)
      -> OwnRef<'r, T>
    {
        let r: &'r mut T = self.write(value);
        unsafe {
            OwnRef::from_raw(
                &mut *(<*mut T>::cast::<MD<T>>(r))
            )
        }
    }
}

pub
trait TupleSlots {
    fn tuple_slots()
      -> Self
    ;
}

impl TupleSlots for () {
    fn tuple_slots()
    {}
}

impl<T, U> TupleSlots for (Slot<T>, Slot<U>) {
    fn tuple_slots()
      -> Self
    {
        (Slot::VACANT, Slot::VACANT)
    }
}
