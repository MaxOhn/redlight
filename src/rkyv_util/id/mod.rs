mod map;

use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    marker::PhantomData,
    num::NonZeroU64,
};

use rkyv::{
    munge::munge,
    niche::niching::{Niching, Zero},
    rancor::Fallible,
    traits::NoUndef,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Place, Portable,
};
use twilight_model::id::Id;

pub use self::map::IdRkyvMap;

/// Used to archive [`Id<T>`] or niche [`ArchivedId<T>`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::id::IdRkyv;
/// use rkyv::with::MapNiche;
/// use twilight_model::id::Id;
///
/// #[derive(Archive)]
/// struct Cached<T> {
///     #[rkyv(with = IdRkyv)]
///     id: Id<T>,
///     // The first `IdRkyv` we use to archive the inner `Id` and the second
///     // `IdRkyv` we use to niche the option for better memory efficiency.
///     #[rkyv(with = MapNiche<IdRkyv, IdRkyv>)]
///     opt: Option<Id<T>>,
/// }
/// ```
pub struct IdRkyv;

/// An archived [`Id`].
#[derive(Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(rkyv::bytecheck::CheckBytes),
    bytecheck(crate = rkyv::bytecheck),
)]
#[repr(C)]
pub struct ArchivedId<T> {
    value: Archived<NonZeroU64>,
    _phantom: PhantomData<fn(T) -> T>,
}

impl<T> ArchiveWith<Id<T>> for IdRkyv {
    type Archived = ArchivedId<T>;
    type Resolver = ();

    fn resolve_with(field: &Id<T>, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedId { value, _phantom } = out);
        field.into_nonzero().resolve(resolver, value);
    }
}

impl<T, S: Fallible + ?Sized> SerializeWith<Id<T>, S> for IdRkyv {
    fn serialize_with(_: &Id<T>, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<T, D> DeserializeWith<ArchivedId<T>, Id<T>, D> for IdRkyv
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        archived: &ArchivedId<T>,
        _: &mut D,
    ) -> Result<Id<T>, <D as Fallible>::Error> {
        Ok(Id::from(*archived))
    }
}

impl<T> Niching<ArchivedId<T>> for IdRkyv {
    unsafe fn is_niched(niched: *const ArchivedId<T>) -> bool {
        unsafe { <Zero as Niching<Archived<NonZeroU64>>>::is_niched(niched.cast()) }
    }

    fn resolve_niched(out: Place<ArchivedId<T>>) {
        <Zero as Niching<Archived<NonZeroU64>>>::resolve_niched(unsafe { out.cast_unchecked() });
    }
}

impl<T> ArchivedId<T> {
    /// Return the inner primitive value.
    pub fn get(self) -> u64 {
        self.into_nonzero().get()
    }

    /// Return the [`NonZeroU64`] representation of the ID.
    pub fn into_nonzero(self) -> NonZeroU64 {
        self.value.into()
    }

    /// Cast an archived ID from one type to another.
    pub const fn cast<New>(self) -> ArchivedId<New> {
        ArchivedId {
            value: self.value,
            _phantom: PhantomData,
        }
    }

    /// Convert an [`ArchivedId<T>`] to an [`Id<T>`].
    pub const fn to_native(self) -> Id<T> {
        // SAFETY: `self.value` is non-zero
        unsafe { Id::new_unchecked(self.value.get()) }
    }

    /// Convert an [`Id<T>`] to an [`ArchivedId<T>`].
    pub const fn from_native(id: Id<T>) -> Self {
        Self {
            value: Archived::<NonZeroU64>::from_native(id.into_nonzero()),
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for ArchivedId<T> {
    fn clone(&self) -> Self {
        *self
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
    fn from(id: Id<T>) -> Self {
        Self::from_native(id)
    }
}

impl<T> From<ArchivedId<T>> for Id<T> {
    fn from(id: ArchivedId<T>) -> Self {
        id.to_native()
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

impl<T> PartialEq<ArchivedId<T>> for Id<T> {
    fn eq(&self, other: &ArchivedId<T>) -> bool {
        other.eq(self)
    }
}

unsafe impl<T> NoUndef for ArchivedId<T> {}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_id() -> Result<(), Error> {
        let id = Id::new(123);
        let bytes = rkyv::to_bytes(With::<_, IdRkyv>::cast(&id))?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &ArchivedId<()> = unsafe { rkyv::access_unchecked(&bytes) };

        #[cfg(feature = "bytecheck")]
        let archived: &ArchivedId<()> = rkyv::access(&bytes)?;

        let deserialized: Id<()> = rkyv::deserialize(With::<_, IdRkyv>::cast(archived))?;

        assert_eq!(id, deserialized);

        Ok(())
    }
}
