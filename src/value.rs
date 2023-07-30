use std::{marker::PhantomData, ops::Deref};

use rkyv::Archive;

use crate::CacheResult;

/// Archived form of a cache entry.
///
/// Implements [`Deref<T>`] so fields and methods of the archived type are easily accessible.
pub struct CachedValue<T> {
    pub(crate) bytes: Vec<u8>,
    phantom: PhantomData<T>,
}

#[cfg(feature = "validation")]
impl<T> CachedValue<T>
where
    T: Archive,
    <T as Archive>::Archived:
        for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>,
{
    pub(crate) fn new(bytes: Vec<u8>) -> CacheResult<Self> {
        if let Err(err) = rkyv::check_archived_root::<T>(bytes.as_slice()) {
            return Err(crate::CacheError::Validation(Box::new(err)));
        }

        Ok(Self {
            bytes,
            phantom: PhantomData,
        })
    }
}

#[cfg(not(feature = "validation"))]
impl<T> CachedValue<T> {
    pub(crate) fn new(bytes: Vec<u8>) -> CacheResult<Self> {
        Ok(Self {
            bytes,
            phantom: PhantomData,
        })
    }
}

impl<T: Archive> Deref for CachedValue<T> {
    type Target = <T as Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        unsafe { rkyv::archived_root::<T>(&self.bytes) }
    }
}
