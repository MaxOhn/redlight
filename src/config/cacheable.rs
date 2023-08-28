use std::time::Duration;

use crate::ser::CacheSerializer;
use rkyv::{ser::Serializer, Fallible, Serialize};

use super::checked::CheckedArchive;

type SerializeResult<T> = Result<
    <<T as Cacheable>::Serializer as CacheSerializer>::Bytes,
    <<T as Cacheable>::Serializer as Fallible>::Error,
>;

/// Trait to configure the serialization and handling of cached entries.
///
/// # Example
/// ```
/// # use std::time::Duration;
/// use rkyv::{Archive, Fallible, Serialize};
/// use rkyv::{ser::serializers::AlignedSerializer, util::AlignedVec};
/// use rkyv::with::RefAsBox;
/// use twilight_redis::config::Cacheable;
///
/// #[derive(Archive, Serialize)]
/// # #[cfg_attr(feature = "validation", archive(check_bytes))]
/// # /*
/// #[archive(check_bytes)] // only if the `validation` feature is enabled
/// # */
/// struct CachedRole<'a> {
///     #[with(RefAsBox)]
///     name: &'a str,
/// }
///
/// impl Cacheable for CachedRole<'_> {
///     // Our `CachedRole` does not contain types that require scratch space
///     // so we can get away with a simpler serializer.
///     // Otherwise, we could use rkyv's `AllocSerializer` or be more
///     // specific with a `CompositeSerializer`.
///     type Serializer = AlignedSerializer<AlignedVec>;
///
///     // We don't want roles to expire.
///     fn expire() -> Option<Duration> { None }
/// }
/// ```
pub trait Cacheable: Sized + Serialize<Self::Serializer> + CheckedArchive {
    /// Serializer used to serialize instances of `Self`.
    ///
    /// When in doubt, use [`AllocSerializer`] with a sensible scratch space size.
    ///
    /// As a very rough rule of thumb:
    ///   - if a [`Vec`] is involved, you want scratch space so [`AllocSerializer`]
    ///   - if a [`String`] with variable length is involved, a [`AlignedSerializer`] should suffice
    ///   - if only simple primitives like integers or bools are involved, you can get away with a [`BufferSerializer`]
    ///
    /// [`AllocSerializer`]: rkyv::ser::serializers::AllocSerializer
    /// [`AlignedSerializer`]: rkyv::ser::serializers::AlignedSerializer
    /// [`BufferSerializer`]: rkyv::ser::serializers::BufferSerializer
    type Serializer: CacheSerializer;

    /// Whether a type should be handled by the cache. Otherwise, it will just be ignored.
    ///
    /// This should always be set to `true`. Otherwise, you should use [`Ignore`](crate::config::Ignore).
    const WANTED: bool = true;

    /// Duration until the cache entry expires and is removed.
    ///
    /// `None` indicates that it will never expire.
    fn expire() -> Option<Duration>;

    /// Serialize `self` with a new default serializer.
    fn serialize(&self) -> SerializeResult<Self> {
        let mut serializer = Self::Serializer::default();
        serializer.serialize_value(self)?;

        Ok(serializer.finish())
    }

    /// Serialize `self` by using a given serializer. Useful when re-using the same serializer.
    fn serialize_with(&self, serializer: &mut Self::Serializer) -> SerializeResult<Self> {
        serializer.serialize_value(self)?;

        Ok(serializer.finish_and_reset())
    }
}
