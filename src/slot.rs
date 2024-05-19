use super::*;

pub
const
fn slot<T>()
  -> Slot<T>
{
    Slot::VACANT
}

pub
const
fn slots<Slots>()
  -> Slots
where
    Slots : TupleSlots,
{
    Slots::TUPLE_SLOTS
}

pub
struct Slot<T>(
    MU<T>,
);

impl<T> Slot<T> {
    pub
    const VACANT: Self = Self(MU::uninit());

    pub
    fn holding<'slot>(self: &'slot mut Slot<T>, value: T)
      -> OwnRef<'slot, T>
    {
        self.0.holding(value)
    }
}

#[extension(pub trait MaybeUninitExt)]
impl<T> MU<T> {
    fn holding<'slot>(&'slot mut self, value: T)
      -> OwnRef<'slot, T>
    {
        let r: &'slot mut T = self.write(value);
        unsafe {
            OwnRef::from_raw(
                <*mut T>::cast::<MD<T>>(r), [],
            )
        }
    }
}

pub
trait TupleSlots {
    const TUPLE_SLOTS: Self;
}

crate::arities::feed_all!(=> impls!);
// where
macro_rules! impls {
    (
        $( $N:ident $($I:ident)* )?
    ) => (
        $( impls! { $($I)* } )?

        impl<$( $N $(, $I)* )?> TupleSlots
            for (
                $( Slot<$N>, $(Slot<$I>),* )?
            )
        {
            const TUPLE_SLOTS: Self = (
                $( Slot::<$N>::VACANT, $(Slot::<$I>::VACANT),* )?
            );
        }
    )
} use impls;
