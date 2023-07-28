use std::{
    borrow::BorrowMut,
    mem::{self, ManuallyDrop},
};

use rkyv::{
    ser::{
        serializers::{AlignedSerializer, BufferSerializer, CompositeSerializer},
        Serializer,
    },
    AlignedVec, Fallible,
};

/// Trait that provides the option to pick and choose a custom serializer.
pub trait CacheSerializer: Default + Fallible + Serializer {
    type Inner: AsRef<[u8]> + Send + Sync;

    /// Finish up serialization by extracting the result from the serializer
    /// and resetting the serializer so that it can be used again.
    fn finish(&mut self) -> Self::Inner;
}

impl<A> CacheSerializer for AlignedSerializer<A>
where
    A: AsRef<[u8]> + BorrowMut<AlignedVec> + Default + Send + Sync,
{
    type Inner = A;

    fn finish(&mut self) -> Self::Inner {
        mem::take(self).into_inner()
    }
}

impl<T> CacheSerializer for BufferSerializer<T>
where
    T: AsRef<[u8]> + AsMut<[u8]> + Default + Send + Sync,
{
    type Inner = T;

    fn finish(&mut self) -> Self::Inner {
        mem::take(self).into_inner()
    }
}

impl<S, C, H> CacheSerializer for CompositeSerializer<S, C, H>
where
    S: CacheSerializer,
    C: Default + Fallible,
    H: Default + Fallible,
{
    type Inner = <S as CacheSerializer>::Inner;

    fn finish(&mut self) -> Self::Inner {
        let ptr = self as *const Self;
        let owned = unsafe { ptr.read() };

        let (mut serializer, scratch, shared) = owned.into_components();
        let inner = serializer.finish();

        let prev = mem::replace(self, Self::new(serializer, scratch, shared));
        let _ = ManuallyDrop::new(prev);

        inner
    }
}
