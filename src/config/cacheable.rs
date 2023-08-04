use crate::ser::CacheSerializer;
use rkyv::{ser::Serializer, AlignedVec, Fallible, Serialize};

use super::checked::CheckedArchive;

type SerializeResult<T> = Result<AlignedVec, <<T as Cacheable>::Serializer as Fallible>::Error>;

/// Trait to configure the serialization and handling of cached entries.
///
/// # Example
/// ```
/// use rkyv::{Archive, Fallible, Serialize};
/// use rkyv::ser::serializers::AlignedSerializer;
/// use rkyv::with::RefAsBox;
/// use rkyv::util::AlignedVec;
/// use twilight_redis::config::Cacheable;
///
/// #[derive(Archive, Serialize)]
/// #[cfg_attr(feature = "validation", archive(check_bytes))]
/// struct CachedRole<'a> {
///     #[with(RefAsBox)]
///     name: &'a str,
/// }
///
/// impl Cacheable for CachedRole<'_> {
///     type Serializer = AlignedSerializer<AlignedVec>;
///
///     fn expire_seconds() -> Option<usize> { None }
/// }
/// ```
pub trait Cacheable: Sized + Serialize<Self::Serializer> + CheckedArchive {
    /// Serializer used to serialize instances of `Self`.
    ///
    /// When in doubt, use [`AllocSerializer`](rkyv::ser::serializers::AllocSerializer) with a sensible scratch space size.
    type Serializer: CacheSerializer;

    /// Whether a type should be handled by the cache. Otherwise, it will just be ignored.
    ///
    /// This should always be set to `true`. Otherwise, you should use [`Ignore`](crate::config::Ignore).
    const WANTED: bool = true;

    /// Amount of seconds until the cache entry expires and is removed.
    /// `None` indicates that it will never expire.
    fn expire_seconds() -> Option<usize>;

    /// Serialize `self` with a new default serializer.
    fn serialize(&self) -> SerializeResult<Self> {
        let mut serializer = Self::Serializer::default();
        serializer.serialize_value(self)?;

        Ok(serializer.finish())
    }

    /// Serialize `self` by using a given serializer. Useful when re-using the same serializer.
    fn serialize_with(&self, serializer: &mut Self::Serializer) -> SerializeResult<Self> {
        serializer.serialize_value(self)?;

        Ok(serializer.finish())
    }
}
