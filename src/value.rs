use std::{marker::PhantomData, ops::Deref};

use rkyv::{rancor::Strategy, seal::Seal, util::AlignedVec, Archive, Archived, Deserialize};

use crate::{config::Cacheable, error::UpdateArchiveError};

/// Archived form of a cache entry.
///
/// Implements [`Deref`] to `T::Archived` so fields and methods of the archived
/// type are easily accessible.
///
/// # Example
///
/// ```
/// use redlight::CachedArchive;
/// use rkyv::{
///     boxed::ArchivedBox, option::ArchivedOption, with::InlineAsBox, Archive, Archived,
///     Deserialize,
/// };
///
/// #[derive(Archive)]
/// struct CachedEntry<'a> {
///     id: u32,
///     #[rkyv(with = InlineAsBox)]
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
///     let id: Archived<u32> = archive.id;
///
///     // The `name` field is archived through the `InlineAsBox` wrapper,
///     // making its archived form an `ArchivedBox`.
///     let name: &ArchivedBox<str> = &archive.name;
///
///     # let archived: ArchivedOption<[u8; 4]> = archive.opt;
///     # let as_ref: Option<&[u8; 4]> = archived.as_ref();
///     # let copied: Option<[u8; 4]> = as_ref.copied();
///     let opt = archive
///         .opt // ArchivedOption<[u8; 4]>
///         .as_ref() // Option<&[u8; 4]>
///         .copied(); // Option<[u8; 4]>
///
///     // Archived types even provide partial deserialization
///     let list: Result<Vec<Inner>, rkyv::rancor::Error> = rkyv::deserialize(&archive.list);
///
///     let first_inner: Option<Result<Inner, rkyv::rancor::Error>> =
///         archive.list.first().map(rkyv::deserialize);
/// }
/// ```
pub struct CachedArchive<T> {
    bytes: AlignedVec<16>,
    phantom: PhantomData<T>,
}

impl<T> CachedArchive<T> {
    pub(crate) const fn new_unchecked(bytes: AlignedVec<16>) -> Self {
        Self {
            bytes,
            phantom: PhantomData,
        }
    }

    /// Consume `self` and return the contained bytes.
    pub fn into_bytes(self) -> AlignedVec<16> {
        self.bytes
    }
}

impl<T: Cacheable> CachedArchive<T> {
    /// Update the contained value by mutating the archive itself.
    ///
    /// This should be preferred over [`update_by_deserializing`] when possible
    /// as it is much more performant.
    ///
    /// # Example
    ///
    /// ```
    /// # use rkyv::Archive;
    /// use redlight::{config::Cacheable, CachedArchive};
    ///
    /// #[derive(Archive)]
    /// struct CachedData {
    ///     num: u32,
    /// }
    ///
    /// impl Cacheable for CachedData {
    ///     # /*
    ///     // ...
    ///     # */
    ///     # type Bytes = [u8; 0];
    ///     # fn expire() -> Option<std::time::Duration> { None }
    ///     # fn serialize_one(&self) -> Result<Self::Bytes, Self::Error> { Ok([]) }
    /// }
    ///
    /// impl rkyv::rancor::Fallible for CachedData {
    ///     type Error = rkyv::rancor::Error;
    /// }
    ///
    /// struct UpdateEvent {
    ///     new_num: u32,
    /// }
    ///
    /// fn handle_archive(archive: &mut CachedArchive<CachedData>, update: &UpdateEvent) {
    ///     archive.update_archive(|sealed| {
    ///         rkyv::munge::munge!(let ArchivedCachedData { mut num } = sealed);
    ///         *num = update.new_num.into()
    ///     }).unwrap();
    /// }
    /// ```
    ///
    /// [`update_by_deserializing`]: CachedArchive::update_by_deserializing
    pub fn update_archive(
        &mut self,
        f: impl FnOnce(Seal<'_, Archived<T>>),
    ) -> Result<(), T::Error> {
        let bytes = self.bytes.as_mut_slice();

        #[cfg(feature = "bytecheck")]
        let sealed = rkyv::access_mut::<Archived<T>, _>(bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let sealed = unsafe { rkyv::access_unchecked_mut::<Archived<T>>(bytes) };

        f(sealed);

        Ok(())
    }

    /// Update the contained value by deserializing the archive, mutating it,
    /// and then serializing again.
    ///
    /// If possible, [`update_archive`] should be used instead as it is much
    /// more performant.
    ///
    /// # Example
    ///
    /// ```
    /// # use rkyv::{Archive, Deserialize, Serialize};
    /// use redlight::{config::Cacheable, CachedArchive};
    /// use rkyv::rancor::Fallible;
    ///
    /// #[derive(Archive, Serialize, Deserialize)]
    /// struct CachedData {
    ///     nums: Vec<u32>,
    /// }
    ///
    /// impl Cacheable for CachedData {
    ///     # /*
    ///     // ...
    ///     # */
    ///     # type Bytes = [u8; 0];
    ///     # fn expire() -> Option<std::time::Duration> { None }
    ///     # fn serialize_one(&self) -> Result<Self::Bytes, Self::Error> { Ok([]) }
    /// }
    ///
    /// impl Fallible for CachedData {
    ///     type Error = rkyv::rancor::Error;
    /// }
    ///
    /// struct UpdateEvent {
    ///     new_nums: Vec<u32>,
    /// }
    ///
    /// fn handle_archive(
    ///     archive: &mut CachedArchive<CachedData>,
    ///     update: &UpdateEvent,
    /// ) -> Result<(), <CachedData as Fallible>::Error> {
    ///     // Updating a Vec like this generally cannot be done through a
    ///     // sealed value so we're using `update_by_deserializing` instead of
    ///     // `update_archive`.
    ///     archive
    ///         .update_by_deserializing(
    ///             |deserialized| deserialized.nums = update.new_nums.clone(),
    ///             &mut (),
    ///         )
    ///         .map_err(rkyv::rancor::Source::new)
    /// }
    /// ```
    ///
    /// [`update_archive`]: CachedArchive::update_archive
    #[allow(clippy::similar_names)]
    pub fn update_by_deserializing<D>(
        &mut self,
        f: impl FnOnce(&mut T),
        deserializer: &mut D,
    ) -> Result<(), UpdateArchiveError<T::Error>>
    where
        T::Archived: Deserialize<T, Strategy<D, T::Error>>,
    {
        let archived: &T::Archived = &*self;

        let mut deserialized: T = rkyv::api::deserialize_using(archived, deserializer)
            .map_err(UpdateArchiveError::Deserialization)?;

        f(&mut deserialized);

        let bytes = deserialized
            .serialize_one()
            .map_err(UpdateArchiveError::Serialization)?;

        self.bytes.clear();
        self.bytes.extend_from_slice(bytes.as_ref());

        Ok(())
    }
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use rkyv::rancor::{BoxedError, Source};

    use crate::{error::CacheError, CacheResult};

    impl<T: Cacheable> CachedArchive<T> {
        /// Create a new [`CachedArchive`].
        ///
        /// # Errors
        ///
        /// Returns an error if the given bytes do not match the archived type.
        pub fn new(bytes: AlignedVec<16>) -> CacheResult<Self> {
            rkyv::access::<Archived<T>, T::Error>(bytes.as_slice())
                .map_err(BoxedError::new)
                .map_err(CacheError::Validation)?;

            Ok(Self::new_unchecked(bytes))
        }
    }
};

impl<T: Archive> Deref for CachedArchive<T> {
    type Target = <T as Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        unsafe { rkyv::access_unchecked::<Archived<T>>(self.bytes.as_slice()) }
    }
}

impl<T> Clone for CachedArchive<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            phantom: PhantomData,
        }
    }
}
