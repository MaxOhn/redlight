use std::{
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

/// Trait that provides the option to pick and choose a custom serializer.
pub trait CacheSerializer: Default + Fallible + Serializer + CacheSerializerExt {
    /// Finish up serialization by extracting the [`AlignedVec`] from the serializer
    /// and resetting the serializer so that it can be used again.
    fn finish(&mut self) -> AlignedVec;
}

/// Auxiliary trait to circumvent the fact that rust currently won't let
/// you specify trait bounds on associated types within trait bounds.
pub trait CacheSerializerExt: Serializer<Error = Self::ErrorExt> {
    type ErrorExt: StdError + 'static;
}

impl<T> CacheSerializerExt for T
where
    T: Serializer,
    <T as Fallible>::Error: StdError,
{
    type ErrorExt = <Self as Fallible>::Error;
}

impl CacheSerializer for AlignedSerializer<AlignedVec> {
    fn finish(&mut self) -> AlignedVec {
        mem::take(self).into_inner()
    }
}

impl CacheSerializer for BufferSerializer<AlignedVec> {
    fn finish(&mut self) -> AlignedVec {
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
    fn finish(&mut self) -> AlignedVec {
        let ptr = self as *const Self;
        let owned = unsafe { ptr.read() };

        let (mut serializer, scratch, shared) = owned.into_components();
        let inner = serializer.finish();

        let prev = mem::replace(self, Self::new(serializer, scratch, shared));
        let _ = ManuallyDrop::new(prev);

        inner
    }
}
