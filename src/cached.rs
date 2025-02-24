use std::{marker::PhantomData, ops::Deref};

use rkyv::{
    rancor::{BoxedError, Strategy},
    seal::Seal,
    util::AlignedVec,
    with::DeserializeWith,
    Deserialize, Portable,
};

use crate::{
    config::{Cacheable, CheckedArchived},
    error::{UpdateArchiveError, ValidationError},
};

/// Archived form of a cache entry.
///
/// Implements [`Deref`] to `T` so fields and methods of the type are easily
/// accessible.
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
/// fn foo(archive: CachedArchive<ArchivedCachedEntry<'_>>) {
///     // The key property of `CachedArchive` is that it derefs
///     // into the generic type.
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

impl<T: CheckedArchived> CachedArchive<T> {
    /// Create a new [`CachedArchive`].
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes do not match the archived type.
    #[cfg(feature = "bytecheck")]
    pub fn new(bytes: AlignedVec<16>) -> Result<Self, ValidationError> {
        rkyv::access::<T, _>(bytes.as_slice())?;

        Ok(Self::new_unchecked(bytes))
    }

    /// Update the contained value by mutating the archive itself.
    ///
    /// This should be preferred over [`update_by_deserializing`] as it is much
    /// more performant. However, since the [`Seal`] api is rather limited,
    /// this is not always possible.
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
    ///     # fn serialize_one<E: rkyv::rancor::Source>(&self) -> Result<Self::Bytes, E> { Ok([]) }
    /// }
    ///
    /// struct UpdateEvent {
    ///     new_num: u32,
    /// }
    ///
    /// fn handle_archive(archive: &mut CachedArchive<ArchivedCachedData>, update: &UpdateEvent) {
    ///     archive.update_archive(|sealed| {
    ///         rkyv::munge::munge!(let ArchivedCachedData { mut num } = sealed);
    ///         *num = update.new_num.into()
    ///     });
    /// }
    /// ```
    ///
    /// [`update_by_deserializing`]: CachedArchive::update_by_deserializing
    pub fn update_archive(&mut self, f: impl FnOnce(Seal<'_, T>)) {
        let bytes = self.bytes.as_mut_slice();

        // SAFETY: The `CachedArchive` is checked upon creation
        let sealed = unsafe { rkyv::access_unchecked_mut::<T>(bytes) };

        f(sealed);
    }
}

impl<T: Portable> CachedArchive<T> {
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
    ///     # fn serialize_one<E: rkyv::rancor::Source>(&self) -> Result<Self::Bytes, E> { Ok([]) }
    /// }
    ///
    /// struct UpdateEvent {
    ///     new_nums: Vec<u32>,
    /// }
    ///
    /// fn handle_archive(
    ///     archive: &mut CachedArchive<ArchivedCachedData>,
    ///     update: &UpdateEvent,
    /// ) {
    ///     // Updating a Vec like this generally cannot be done through a
    ///     // sealed value so we're using `update_by_deserializing` instead of
    ///     // `update_archive`.
    ///     archive.update_by_deserializing(
    ///         |deserialized| deserialized.nums = update.new_nums.clone(),
    ///         &mut (),
    ///     ).unwrap()
    /// }
    /// ```
    ///
    /// [`update_archive`]: CachedArchive::update_archive
    pub fn update_by_deserializing<C, D>(
        &mut self,
        f: impl FnOnce(&mut C),
        deserializer: &mut D,
    ) -> Result<(), UpdateArchiveError>
    where
        C: Cacheable,
        T: Deserialize<C, Strategy<D, BoxedError>>,
    {
        // clippy disapproves the usage of "deserializer" and "deserialized"
        #![allow(clippy::similar_names)]

        let archived: &T = &*self;

        let mut deserialized: C = rkyv::api::deserialize_using(archived, deserializer)
            .map_err(UpdateArchiveError::Deserialization)?;

        f(&mut deserialized);

        self.bytes.clear();

        deserialized
            .serialize_into(&mut self.bytes)
            .map_err(UpdateArchiveError::Serialization)?;

        Ok(())
    }

    /// Convenience method to deserialize the archive.
    pub fn try_deserialize<U, D, E>(&self, deserializer: &mut D) -> Result<U, E>
    where
        T: Deserialize<U, Strategy<D, E>>,
    {
        self.deserialize(Strategy::wrap(deserializer))
    }

    /// Convenience method to deserialize the archive with a given wrapper `W`.
    pub fn try_deserialize_with<W, U, D, E>(&self, deserializer: &mut D) -> Result<U, E>
    where
        W: DeserializeWith<T, U, Strategy<D, E>>,
    {
        W::deserialize_with(self, Strategy::wrap(deserializer))
    }
}

impl<T: Portable> Deref for CachedArchive<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { rkyv::access_unchecked::<T>(self.bytes.as_slice()) }
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
