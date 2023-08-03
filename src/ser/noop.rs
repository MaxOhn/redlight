use std::convert::Infallible;

use rkyv::{
    ser::Serializer, AlignedVec, Archive, ArchiveUnsized, Fallible, Serialize, SerializeUnsized,
};

use crate::ser::CacheSerializer;

/// Serializer that doesn't serialize anything. Used by [`Ignore`](crate::config::Ignore).
#[derive(Default)]
pub struct NoopSerializer;

impl Fallible for NoopSerializer {
    type Error = Infallible;
}

impl Serializer for NoopSerializer {
    fn pos(&self) -> usize {
        0
    }

    fn write(&mut self, _: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn pad(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }

    fn align(&mut self, _: usize) -> Result<usize, Self::Error> {
        Ok(self.pos())
    }

    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        _: &T,
        _: T::Resolver,
    ) -> Result<usize, Self::Error> {
        Ok(self.pos())
    }

    fn serialize_value<T: Serialize<Self>>(&mut self, _: &T) -> Result<usize, Self::Error> {
        Ok(self.pos())
    }

    unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
        &mut self,
        _: &T,
        _: usize,
        _: T::MetadataResolver,
    ) -> Result<usize, Self::Error> {
        Ok(self.pos())
    }

    fn serialize_unsized_value<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        _: &T,
    ) -> Result<usize, Self::Error> {
        Ok(self.pos())
    }
}

impl CacheSerializer for NoopSerializer {
    fn finish(&mut self) -> AlignedVec {
        AlignedVec::new()
    }
}
