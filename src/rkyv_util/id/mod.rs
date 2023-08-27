mod map;

use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    marker::PhantomData,
    num::NonZeroU64,
};

use rkyv::{
    out_field,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Fallible,
};
use twilight_model::id::Id;

pub use self::map::{ArchivedIdOption, IdRkyvMap};

/// Used to archive [`Id<T>`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use twilight_model::id::Id;
/// use twilight_redis::rkyv_util::id::IdRkyv;
///
/// #[derive(Archive)]
/// struct Cached<T> {
///     #[with(IdRkyv)]
///     id: Id<T>,
/// }
/// ```
pub struct IdRkyv;

/// An archived [`Id<T>`].
pub struct ArchivedId<T> {
    value: Archived<NonZeroU64>,
    phantom: PhantomData<fn(T) -> T>,
}

impl<T> ArchivedId<T> {
    /// Return the inner primitive value.
    pub fn get(self) -> u64 {
        self.into_nonzero().get()
    }

    /// Return the [`NonZeroU64`] representation of the ID.
    pub fn into_nonzero(self) -> NonZeroU64 {
        // the .into() is necessary in case the `archive_le` or `archive_be`
        // features are enabled in rkyv
        #[allow(clippy::useless_conversion)]
        self.value.into()
    }
}

impl<T> Clone for ArchivedId<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom: PhantomData,
        }
    }
}

impl<T> Copy for ArchivedId<T> {}

impl<T> Display for ArchivedId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.value, f)
    }
}

impl<T> Debug for ArchivedId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self, f)
    }
}

impl<T> From<Id<T>> for ArchivedId<T> {
    fn from(value: Id<T>) -> Self {
        Self {
            // the .into() is necessary in case the `archive_le` or `archive_be`
            // features are enabled in rkyv
            #[allow(clippy::useless_conversion)]
            value: value.into_nonzero().into(),
            phantom: PhantomData,
        }
    }
}

impl<T> From<ArchivedId<T>> for Id<T> {
    fn from(id: ArchivedId<T>) -> Self {
        // the `from` is necessary in case the `archive_le` or `archive_be`
        // features are enabled in rkyv
        #[allow(clippy::useless_conversion)]
        Id::from(NonZeroU64::from(id.value))
    }
}

impl<T> PartialEq for ArchivedId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for ArchivedId<T> {}

impl<T> PartialEq<Id<T>> for ArchivedId<T> {
    fn eq(&self, other: &Id<T>) -> bool {
        self.value == other.into_nonzero()
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use std::ptr::addr_of;

    use rkyv::{bytecheck::NonZeroCheckError, CheckBytes};

    impl<C: ?Sized, T> CheckBytes<C> for ArchivedId<T> {
        type Error = NonZeroCheckError;

        unsafe fn check_bytes<'bytecheck>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'bytecheck Self, Self::Error> {
            Archived::<NonZeroU64>::check_bytes(addr_of!((*value).value), context)?;

            Ok(&*value)
        }
    }
};

impl<T> ArchiveWith<Id<T>> for IdRkyv {
    type Archived = ArchivedId<T>;
    type Resolver = ();

    unsafe fn resolve_with(
        id: &Id<T>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.value);
        id.into_nonzero().resolve(pos + fp, resolver, fo);
    }
}

impl<T, S: Fallible + ?Sized> SerializeWith<Id<T>, S> for IdRkyv {
    fn serialize_with(_: &Id<T>, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<T, D: Fallible + ?Sized> DeserializeWith<ArchivedId<T>, Id<T>, D> for IdRkyv {
    fn deserialize_with(
        archived: &ArchivedId<T>,
        deserializer: &mut D,
    ) -> Result<Id<T>, <D as Fallible>::Error> {
        archived.deserialize(deserializer)
    }
}

impl<D: Fallible + ?Sized, T> Deserialize<Id<T>, D> for ArchivedId<T> {
    fn deserialize(&self, _: &mut D) -> Result<Id<T>, <D as Fallible>::Error> {
        Ok(Id::from(self.into_nonzero()))
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_id() {
        type Wrapper = With<Id<()>, IdRkyv>;

        let id = Id::new(123);
        let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&id)).unwrap();

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

        let deserialized: Id<()> = archived.deserialize(&mut Infallible).unwrap();

        assert_eq!(id, deserialized);
    }
}
