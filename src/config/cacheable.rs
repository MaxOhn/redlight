use std::time::Duration;

use rkyv::{rancor::Source, util::AlignedVec, Archive};

use super::CheckedArchived;

/// Trait to configure the serialization and handling of cached entries.
///
/// # Example
/// ```
/// # use std::time::Duration;
/// use redlight::config::Cacheable;
/// use rkyv::{rancor::Source, util::AlignedVec, with::InlineAsBox, Archive, Serialize};
///
/// #[derive(Archive, Serialize)]
/// struct CachedRole<'a> {
///     #[rkyv(with = InlineAsBox)]
///     name: &'a str,
/// }
///
/// impl Cacheable for CachedRole<'_> {
///     // The type that `serialize_one` returns upon successful serialization.
///     type Bytes = AlignedVec<8>;
///
///     // We don't want roles to expire.
///     fn expire() -> Option<Duration> {
///         None
///     }
///
///     fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
///         // Serializing our `CachedRole` requires neither an `Allocator`
///         // nor a `Sharing` trait bound on the serializer so we can simply
///         // use an `AlignedVec`.
///         // Alternatively, the easiest way to serialize is `rkyv::to_bytes`.
///         let mut bytes = AlignedVec::<8>::new();
///         rkyv::api::serialize_using(self, &mut bytes)?;
///
///         Ok(bytes)
///     }
/// }
/// ```
pub trait Cacheable: Archive<Archived: CheckedArchived> + Sized {
    /// The resulting byte buffer after serialization.
    type Bytes: AsRef<[u8]>;

    /// Whether a type should be handled by the cache. Otherwise, it will just
    /// be ignored.
    ///
    /// This should always be set to `true`. Otherwise, you should use
    /// [`Ignore`](crate::config::Ignore).
    const WANTED: bool = true;

    /// Duration until the cache entry expires and is removed.
    ///
    /// `None` indicates that it will never expire.
    fn expire() -> Option<Duration>;

    /// How to serialize this type into bytes.
    ///
    /// Tips:
    /// - General purpose: put [`AlignedVec`] as `Self::Bytes` and use
    ///   [`rkyv::to_bytes`]
    /// - More flexible writer: use [`rkyv::api::high::to_bytes_in`]
    /// - Avoid unnecessary [`Allocator`] and [`Sharing`] serializers: use
    ///   [`rkyv::api::serialize_using`]
    ///
    /// [`AlignedVec`]: rkyv::util::AlignedVec
    /// [`Allocator`]: rkyv::ser::Allocator
    /// [`Sharing`]: rkyv::ser::Sharing
    fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E>;

    /// How to serialize this type into an [`AlignedVec`].
    ///
    /// By default this method utilizes [`serialize_one`]. If [`Self::Bytes`]
    /// is an [`AlignedVec`], this method could be specialized to improve
    /// performance.
    ///
    /// [`serialize_one`]: Cacheable::serialize_one
    fn serialize_into<E: Source, const N: usize>(
        &self,
        bytes: &mut AlignedVec<N>,
    ) -> Result<(), E> {
        let serialized = self.serialize_one()?;
        bytes.extend_from_slice(serialized.as_ref());

        Ok(())
    }

    /// Returns a serializer capable of serializing multiple instances in a row.
    ///
    /// This serializer is able to keep state inbetween serializations to
    /// potentially improve performance.
    ///
    /// Unless implemented manually, the default serializer will just use
    /// [`serialize_one`] repeatedly.
    ///
    /// [`serialize_one`]: Cacheable::serialize_one
    fn serialize_many() -> impl SerializeMany<Self> {
        SerializeOneByOne
    }
}

/// A serializer to serialize multiple instances of `C` in a row.
pub trait SerializeMany<C: Cacheable> {
    /// The resulting byte buffer after serialization.
    type Bytes: AsRef<[u8]>;

    /// Serialize the next instance.
    fn serialize_next<E: Source>(&mut self, next: &C) -> Result<Self::Bytes, E>;
}

struct SerializeOneByOne;

impl<C: Cacheable> SerializeMany<C> for SerializeOneByOne {
    type Bytes = C::Bytes;

    fn serialize_next<E: Source>(&mut self, next: &C) -> Result<Self::Bytes, E> {
        next.serialize_one()
    }
}
