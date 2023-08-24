mod map;

use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    marker::PhantomData,
    num::NonZeroU64,
};

use rkyv::{
    out_field,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Fallible,
};
use twilight_model::id::Id;

pub use self::map::IdRkyvMap;

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

pub struct ArchivedId<T> {
    value: NonZeroU64,
    phantom: PhantomData<fn(T) -> T>,
}

impl<T> ArchivedId<T> {
    pub fn get(self) -> u64 {
        self.value.get()
    }

    pub fn into_nonzero(self) -> NonZeroU64 {
        self.value
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
            value: value.into_nonzero(),
            phantom: PhantomData,
        }
    }
}

impl<T> From<ArchivedId<T>> for Id<T> {
    fn from(id: ArchivedId<T>) -> Self {
        Id::from(id.value)
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
impl<C: ?Sized, T> rkyv::CheckBytes<C> for ArchivedId<T> {
    type Error = rkyv::bytecheck::NonZeroCheckError;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        if *u64::check_bytes(value.cast(), context)? == 0 {
            Err(rkyv::bytecheck::NonZeroCheckError::IsZero)
        } else {
            Ok(&*value)
        }
    }
}

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
    fn deserialize_with(id: &ArchivedId<T>, _: &mut D) -> Result<Id<T>, <D as Fallible>::Error> {
        Ok(Id::from(id.into_nonzero()))
    }
}
