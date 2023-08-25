use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    mem,
    num::NonZeroU64,
};

use rkyv::{
    boxed::{ArchivedBox, BoxResolver},
    out_field,
    ser::Serializer,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    ArchiveUnsized, Fallible,
};
use twilight_model::id::Id;

/// Used to archive `Option<Id<T>>`, `Vec<Id<T>>`, `&[Id<T>]`, and `Box<[Id<T>]>` more efficiently
/// than with [`Map`](rkyv::with::Map) and [`IdRkyv`](crate::rkyv_util::id::IdRkyv).
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use twilight_model::id::Id;
/// use twilight_redis::rkyv_util::id::IdRkyvMap;
///
/// #[derive(Archive)]
/// struct Cached<'a, T> {
///     #[with(IdRkyvMap)]
///     id_opt: Option<Id<T>>,
///     #[with(IdRkyvMap)]
///     id_vec: Vec<Id<T>>,
///     #[with(IdRkyvMap)]
///     id_slice: &'a [Id<T>],
///     #[with(IdRkyvMap)]
///     id_box: Box<[Id<T>]>,
/// }
/// ```
pub struct IdRkyvMap;

// IdRkyvMap for Options

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ArchivedIdOption {
    inner: u64,
}

impl ArchivedIdOption {
    pub fn to_nonzero_option(self) -> Option<NonZeroU64> {
        if self.inner != 0 {
            // SAFETY: NonZero types have the same memory layout and bit patterns as
            // their integer counterparts, regardless of endianness
            let as_nonzero = unsafe { *(&self.inner as *const _ as *const NonZeroU64) };

            Some(as_nonzero)
        } else {
            None
        }
    }

    pub fn to_id_option<T>(self) -> Option<Id<T>> {
        self.to_nonzero_option().map(Id::from)
    }

    pub unsafe fn resolve_from_id<T>(field: Option<Id<T>>, out: *mut Self) {
        let (_, fo) = out_field!(out.inner);

        if let Some(id) = field {
            fo.write(id.get());
        } else {
            fo.write(0);
        }
    }
}

impl Debug for ArchivedIdOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.to_nonzero_option() {
            Some(ref nonzero) => f.debug_tuple("Some").field(nonzero).finish(),
            None => f.write_str("None"),
        }
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use std::convert::Infallible;

    use rkyv::CheckBytes;

    impl<C: ?Sized> CheckBytes<C> for ArchivedIdOption {
        type Error = Infallible;

        unsafe fn check_bytes<'a>(value: *const Self, _: &mut C) -> Result<&'a Self, Self::Error> {
            Ok(&*value)
        }
    }
};

impl<T> ArchiveWith<Option<Id<T>>> for IdRkyvMap {
    type Archived = ArchivedIdOption;
    type Resolver = ();

    unsafe fn resolve_with(
        id: &Option<Id<T>>,
        _: usize,
        _: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedIdOption::resolve_from_id(*id, out);
    }
}

impl<S: Fallible + ?Sized, T> SerializeWith<Option<Id<T>>, S> for IdRkyvMap {
    fn serialize_with(_: &Option<Id<T>>, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized, T> DeserializeWith<ArchivedIdOption, Option<Id<T>>, D> for IdRkyvMap {
    #[inline]
    fn deserialize_with(archived: &ArchivedIdOption, _: &mut D) -> Result<Option<Id<T>>, D::Error> {
        Ok(archived.to_id_option())
    }
}

// IdRkyvMap for Vecs

impl<T> ArchiveWith<Vec<Id<T>>> for IdRkyvMap {
    type Archived = ArchivedBox<<[NonZeroU64] as ArchiveUnsized>::Archived>;
    type Resolver = BoxResolver<<[NonZeroU64] as ArchiveUnsized>::MetadataResolver>;

    unsafe fn resolve_with(
        ids: &Vec<Id<T>>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let slice = ids_to_nonzeros(ids.as_slice());
        ArchivedBox::resolve_from_ref(slice, pos, resolver, out);
    }
}

impl<S: Serializer + Fallible, T> SerializeWith<Vec<Id<T>>, S> for IdRkyvMap {
    fn serialize_with(
        ids: &Vec<Id<T>>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        let slice = ids_to_nonzeros(ids.as_slice());

        unsafe { ArchivedBox::serialize_copy_from_slice(slice, serializer) }
    }
}

impl<D: Fallible + ?Sized, T>
    DeserializeWith<<IdRkyvMap as ArchiveWith<Vec<Id<T>>>>::Archived, Vec<Id<T>>, D> for IdRkyvMap
{
    fn deserialize_with(
        archived: &<IdRkyvMap as ArchiveWith<Vec<Id<T>>>>::Archived,
        _: &mut D,
    ) -> Result<Vec<Id<T>>, <D as Fallible>::Error> {
        Ok(nonzeros_to_ids(archived).to_owned())
    }
}

// IdRkyvMap for Box<[Id<T>]>

impl<T> ArchiveWith<Box<[Id<T>]>> for IdRkyvMap {
    type Archived = ArchivedBox<<[NonZeroU64] as ArchiveUnsized>::Archived>;
    type Resolver = BoxResolver<<[NonZeroU64] as ArchiveUnsized>::MetadataResolver>;

    unsafe fn resolve_with(
        ids: &Box<[Id<T>]>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let slice = ids_to_nonzeros(ids.as_ref());
        ArchivedBox::resolve_from_ref(slice, pos, resolver, out);
    }
}

impl<S: Serializer + Fallible, T> SerializeWith<Box<[Id<T>]>, S> for IdRkyvMap {
    fn serialize_with(
        ids: &Box<[Id<T>]>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        let slice = ids_to_nonzeros(ids.as_ref());

        unsafe { ArchivedBox::serialize_copy_from_slice(slice, serializer) }
    }
}

impl<D: Fallible + ?Sized, T>
    DeserializeWith<<IdRkyvMap as ArchiveWith<Box<[Id<T>]>>>::Archived, Box<[Id<T>]>, D>
    for IdRkyvMap
{
    fn deserialize_with(
        archived: &<IdRkyvMap as ArchiveWith<Vec<Id<T>>>>::Archived,
        _: &mut D,
    ) -> Result<Box<[Id<T>]>, <D as Fallible>::Error> {
        Ok(Box::from(nonzeros_to_ids(archived)))
    }
}

// IdRkyvMap for slices

impl<T> ArchiveWith<&[Id<T>]> for IdRkyvMap {
    type Archived = ArchivedBox<<[NonZeroU64] as ArchiveUnsized>::Archived>;
    type Resolver = BoxResolver<<[NonZeroU64] as ArchiveUnsized>::MetadataResolver>;

    unsafe fn resolve_with(
        ids: &&[Id<T>],
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let slice = ids_to_nonzeros(ids);
        ArchivedBox::resolve_from_ref(slice, pos, resolver, out);
    }
}

impl<S: Serializer + Fallible, T> SerializeWith<&[Id<T>], S> for IdRkyvMap {
    fn serialize_with(
        ids: &&[Id<T>],
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        let slice = ids_to_nonzeros(ids);

        unsafe { ArchivedBox::serialize_copy_from_slice(slice, serializer) }
    }
}

fn ids_to_nonzeros<T>(ids: &[Id<T>]) -> &[NonZeroU64] {
    // SAFETY: Id<T> is a transparent wrapper of NonZeroU64
    unsafe { mem::transmute(ids) }
}

fn nonzeros_to_ids<T>(ids: &[NonZeroU64]) -> &[Id<T>] {
    // SAFETY: Id<T> is a transparent wrapper of NonZeroU64
    unsafe { mem::transmute(ids) }
}
