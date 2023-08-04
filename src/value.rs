use std::{error::Error as StdError, marker::PhantomData, ops::Deref, pin::Pin};

use rkyv::{ser::Serializer, Archive, Deserialize, Fallible};

use crate::{config::Cacheable, error::UpdateArchiveError, ser::CacheSerializer};

/// Archived form of a cache entry.
///
/// Implements [`Deref`] to `T::Archived` so fields and methods of the archived type are easily accessible.
pub struct CachedArchive<T> {
    bytes: Box<[u8]>,
    phantom: PhantomData<T>,
}

impl<T> CachedArchive<T> {
    pub(crate) fn new_unchecked(bytes: Box<[u8]>) -> Self {
        Self {
            bytes,
            phantom: PhantomData,
        }
    }

    /// Consume `self` and return the contained bytes.
    pub fn into_bytes(self) -> Box<[u8]> {
        self.bytes
    }
}

impl<T: Archive> CachedArchive<T> {
    /// Update the contained value by mutating the archive itself.
    ///
    /// This should be preferred over [`update_by_deserializing`] when possible
    /// as it is much more performant.
    ///
    /// [`update_by_deserializing`]: CachedArchive::update_by_deserializing
    pub fn update_archive(&mut self, f: impl FnOnce(Pin<&mut T::Archived>)) {
        let bytes = self.bytes.as_mut();
        let pin = unsafe { rkyv::archived_root_mut::<T>(Pin::new(bytes)) };
        f(pin);
    }
}

impl<T: Cacheable> CachedArchive<T> {
    /// Update the contained value by deserializing the archive,
    /// mutating it, and then serializing again.
    ///
    /// If possible, [`update_archive`] should be used instead as it is much more performant.
    ///
    /// [`update_archive`]: CachedArchive::update_archive
    pub fn update_by_deserializing<D>(
        &mut self,
        f: impl FnOnce(&mut T),
        deserializer: &mut D,
    ) -> Result<(), UpdateArchiveError<<D as Fallible>::Error, <T::Serializer as Fallible>::Error>>
    where
        D: Fallible,
        D::Error: StdError,
        T::Archived: Deserialize<T, D>,
    {
        let archived: &T::Archived = &*self;

        let mut deserialized: T = archived
            .deserialize(deserializer)
            .map_err(UpdateArchiveError::Deserialization)?;

        f(&mut deserialized);
        let mut serializer = T::Serializer::default();

        serializer
            .serialize_value(&deserialized)
            .map_err(UpdateArchiveError::Serialization)?;

        let bytes = serializer.finish();
        self.bytes = bytes.into_boxed_slice();

        Ok(())
    }
}

#[cfg(feature = "validation")]
impl<T> CachedArchive<T>
where
    T: Archive,
    <T as Archive>::Archived:
        for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>,
{
    pub(crate) fn new(bytes: Box<[u8]>) -> crate::CacheResult<Self> {
        rkyv::check_archived_root::<T>(bytes.as_ref())
            .map_err(|e| crate::CacheError::Validation(Box::new(e)))?;

        Ok(Self::new_unchecked(bytes))
    }
}

impl<T: Archive> Deref for CachedArchive<T> {
    type Target = <T as Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        unsafe { rkyv::archived_root::<T>(self.bytes.as_ref()) }
    }
}
