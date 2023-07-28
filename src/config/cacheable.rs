use std::fmt::Debug;

use crate::ser::CacheSerializer;
use rkyv::{ser::Serializer, Fallible, Serialize};

use super::checked::CheckedArchive;

type SerializeResult<T> = Result<
    <<T as Cacheable>::Serializer as CacheSerializer>::Inner,
    <<T as Cacheable>::Serializer as Fallible>::Error,
>;

/// Trait to configure the serialization of cached entries.
///
/// # Example
/// ```
/// use rkyv::{ser::serializers::AllocSerializer, with::RefAsBox, Archive, Fallible, Serialize};
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
///     type Serializer = AllocSerializer<0>;
///     type SerializeError = <Self::Serializer as Fallible>::Error;
/// }
/// ```
pub trait Cacheable: Sized + Serialize<Self::Serializer> + CheckedArchive {
    /// Serializer used to serialize instances of `Self`.
    ///
    /// When in doubt, use rkyv's [`AllocSerializer`](rkyv::ser::serializers::AllocSerializer) with a sensible scratch space size.
    type Serializer: CacheSerializer<Error = Self::SerializeError>;

    /// Error of `Self::Serializer`. This should generally be set to `<Self::Serializer as Fallible>::Error`.
    ///
    /// The only reason this type exists is due to rust's current lack of support
    /// for providing trait bounds on associated types.
    type SerializeError: Debug;

    /// Whether a type should be handled by the cache. Otherwise, it will just be ignored.
    ///
    /// This should always be set to `true`. Otherwise, you should use [`Ignore`](crate::config::Ignore).
    const WANTED: bool = true;

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
