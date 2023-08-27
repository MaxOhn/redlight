use std::{
    borrow::BorrowMut,
    error::Error as StdError,
    mem::{self, ManuallyDrop},
};

use rkyv::{
    ser::{
        serializers::{AlignedSerializer, BufferSerializer, CompositeSerializer},
        Serializer,
    },
    AlignedVec, Fallible,
};

use super::SerializerExt;

/// Trait that provides the option to pick and choose a custom serializer.
///
/// Out of the box, the trait is implemented for rkyv's [`AlignedSerializer`],
/// [`BufferSerializer`], and [`CompositeSerializer`].
pub trait CacheSerializer: Default + Serializer + SerializerExt {
    /// The container on which the serializer operates.
    type Bytes: AsRef<[u8]>;

    /// Finish up serialization by extracting `Self::Bytes` from the serializer.
    fn finish(self) -> Self::Bytes;

    /// Finish up serialization by extracting `Self::Bytes` from the serializer
    /// and resetting the serializer so that it can be used again.
    fn finish_and_reset(&mut self) -> Self::Bytes;
}

impl<A: AsRef<[u8]> + BorrowMut<AlignedVec> + Default> CacheSerializer for AlignedSerializer<A> {
    type Bytes = A;

    fn finish(self) -> Self::Bytes {
        self.into_inner()
    }

    fn finish_and_reset(&mut self) -> Self::Bytes {
        mem::take(self).into_inner()
    }
}

impl<A: AsMut<[u8]> + AsRef<[u8]> + Default> CacheSerializer for BufferSerializer<A> {
    type Bytes = A;

    fn finish(self) -> Self::Bytes {
        self.into_inner()
    }

    fn finish_and_reset(&mut self) -> Self::Bytes {
        mem::take(self).into_inner()
    }
}

impl<S, C, H> CacheSerializer for CompositeSerializer<S, C, H>
where
    S: CacheSerializer,
    C: Default + Fallible,
    <C as Fallible>::Error: StdError,
    H: Default + Fallible,
    <H as Fallible>::Error: StdError,
{
    type Bytes = <S as CacheSerializer>::Bytes;

    fn finish(self) -> Self::Bytes {
        self.into_serializer().finish()
    }

    fn finish_and_reset(&mut self) -> Self::Bytes {
        let ptr = self as *const Self;

        // SAFETY: We acquire temporary ownership of self.
        // This is ok because the mutable reference is not accessed in the meanwhile.
        // In order to prevent dropping self twice, once we're done with the owned
        // instance, we get rid of it without dropping.
        let owned = unsafe { ptr.read() };

        let (mut serializer, scratch, shared) = owned.into_components();
        let inner = serializer.finish_and_reset();

        let prev = mem::replace(self, Self::new(serializer, scratch, shared));
        let _ = ManuallyDrop::new(prev);

        inner
    }
}
