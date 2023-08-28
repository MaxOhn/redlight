use std::{error::Error as StdError, marker::PhantomData, ops::Deref, pin::Pin};

use rkyv::{ser::Serializer, Archive, Deserialize, Fallible};

use crate::{config::Cacheable, error::UpdateArchiveError, ser::CacheSerializer};

/// Archived form of a cache entry.
///
/// Implements [`Deref`] to `T::Archived` so fields and methods of the archived type are easily accessible.
///
/// # Example
///
/// ```
/// use rkyv::{boxed::ArchivedBox, option::ArchivedOption, with::RefAsBox};
/// use rkyv::{Archive, Archived, Deserialize, Infallible};
/// use twilight_redis::CachedArchive;
///
/// #[derive(Archive)]
/// struct CachedEntry<'a> {
///     id: u32,
///     #[with(RefAsBox)]
///     name: &'a str,
///     opt: Option<[u8; 4]>,
///     list: Vec<Inner>,
/// }
///
/// #[derive(Archive, Deserialize)]
/// struct Inner {
///     field: String,
/// }
///
/// fn foo(archive: CachedArchive<CachedEntry<'_>>) {
///     // The key property of `CachedArchive` is that it derefs
///     // into the archived form of the generic type.
///     let _: &ArchivedCachedEntry<'_> = &archive;
///
///     // Unless rkyv's `archived_le` or `archived_be` features are enabled,
///     // `Archived<u32>` is just a `u32`.
///     let id: Archived<u32> = archive.id;
///
///     // The `name` field is archived through the `RefAsBox` wrapper,
///     // making its archived form an `ArchivedBox`.
///     let name: &ArchivedBox<str> = &archive.name;
///
///     let opt = archive.opt // ArchivedOption<[u8; 4]>
///         .as_ref()         // Option<&[u8; 4]>
///         .copied();        // Option<[u8; 4]>
///
///     // Archived types even provide partial deserialization
///     let list: Vec<Inner> = archive.list // ArchivedVec<ArchivedInner>
///         .deserialize(&mut Infallible)
///         .unwrap();
///
///     let first_inner: Option<Inner> = archive.list
///         .first()
///         .map(|inner| inner.deserialize(&mut Infallible).unwrap());
/// }
/// ```
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
    /// # Example
    ///
    /// ```
    /// # use rkyv::Archive;
    /// use twilight_redis::CachedArchive;
    ///
    /// #[derive(Archive)]
    /// struct CachedData {
    ///     num: u32,
    /// }
    ///
    /// struct UpdateEvent {
    ///     new_num: u32,
    /// }
    ///
    /// fn handle_archive(archive: &mut CachedArchive<CachedData>, update: &UpdateEvent) {
    ///     archive.update_archive(|mut pinned| pinned.num = update.new_num);
    /// }
    /// ```
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
    /// Returns a boxed [`std::error::Error`] if either deserialization or serialization failed.
    ///
    /// # Example
    ///
    /// ```
    /// use std::error::Error;
    /// # use rkyv::{Archive, Deserialize, Serialize};
    /// use rkyv::Infallible;
    /// use twilight_redis::{config::Cacheable, CachedArchive};
    ///
    /// #[derive(Archive, Serialize, Deserialize)]
    /// # #[cfg_attr(feature = "validation", archive(check_bytes))]
    /// # /*
    /// #[archive(check_bytes)] // only if the `validation` feature is enabled
    /// # */
    /// struct CachedData {
    ///     nums: Vec<u32>,
    /// }
    ///
    /// impl Cacheable for CachedData {
    ///     # /*
    ///     // ...
    ///     # */
    ///     # type Serializer = rkyv::ser::serializers::AllocSerializer<32>;
    ///     # fn expire() -> Option<std::time::Duration> { None }
    /// }
    ///
    /// struct UpdateEvent {
    ///     new_nums: Vec<u32>,
    /// }
    ///
    /// fn handle_archive(archive: &mut CachedArchive<CachedData>, update: &UpdateEvent) -> Result<(), Box<dyn Error>> {
    ///     // Updating a Vec like this generally cannot be done through a pinned mutable reference
    ///     // so we're using `update_by_deserializing` instead of `update_archive`.
    ///     archive.update_by_deserializing(
    ///         |deserialized| deserialized.nums = update.new_nums.clone(),
    ///         &mut Infallible,
    ///     )
    /// }
    /// ```
    ///
    /// [`update_archive`]: CachedArchive::update_archive
    pub fn update_by_deserializing<D>(
        &mut self,
        f: impl FnOnce(&mut T),
        deserializer: &mut D,
    ) -> Result<(), Box<dyn StdError>>
    where
        D: Fallible,
        D::Error: StdError,
        T::Archived: Deserialize<T, D>,
    {
        let archived: &T::Archived = &*self;

        let mut deserialized: T = archived
            .deserialize(deserializer)
            .map_err(UpdateArchiveError::<_, <T::Serializer as Fallible>::Error>::Deserialization)
            .map_err(Box::new)?;

        f(&mut deserialized);
        let mut serializer = T::Serializer::default();

        serializer
            .serialize_value(&deserialized)
            .map_err(UpdateArchiveError::<<D as Fallible>::Error, _>::Serialization)
            .map_err(Box::new)?;

        let bytes = serializer.finish();
        self.bytes = Box::from(bytes.as_ref());

        Ok(())
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use rkyv::{validation::validators::DefaultValidator, CheckBytes};

    use crate::{CacheError, CacheResult};

    impl<T> CachedArchive<T>
    where
        T: Archive,
        <T as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>>,
    {
        pub(crate) fn new(bytes: Box<[u8]>) -> CacheResult<Self> {
            rkyv::check_archived_root::<T>(bytes.as_ref())
                .map_err(|e| CacheError::Validation(Box::new(e)))?;

            Ok(Self::new_unchecked(bytes))
        }
    }
};

impl<T: Archive> Deref for CachedArchive<T> {
    type Target = <T as Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        unsafe { rkyv::archived_root::<T>(self.bytes.as_ref()) }
    }
}
