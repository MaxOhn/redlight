use std::{marker::PhantomData, ops::Deref};

use rkyv::Archive;

/// Archived form of a cache entry.
///
/// Implements [`Deref<T>`] so fields and methods of the archived type are easily accessible.
pub struct CachedValue<T> {
    bytes: Box<[u8]>,
    phantom: PhantomData<T>,
}

impl<T> CachedValue<T> {
    pub(crate) fn new_unchecked(bytes: Box<[u8]>) -> Self {
        Self {
            bytes,
            phantom: PhantomData,
        }
    }

    pub(crate) fn into_bytes(self) -> Box<[u8]> {
        self.bytes
    }
}

#[cfg(feature = "validation")]
impl<T> CachedValue<T>
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

impl<T: Archive> Deref for CachedValue<T> {
    type Target = <T as Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        unsafe { rkyv::archived_root::<T>(self.bytes.as_ref()) }
    }
}
