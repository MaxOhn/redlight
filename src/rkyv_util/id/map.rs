use std::{mem, num::NonZeroU64, ptr, slice};

use rkyv::{
    niche::niched_option::NichedOption,
    rancor::Fallible,
    ser::{Allocator, Writer, WriterExt},
    vec::{ArchivedVec, VecResolver},
    with::{ArchiveWith, DeserializeWith, Map, MapNiche, SerializeWith, With},
    Archive, Archived, Deserialize, Place,
};
use twilight_model::id::Id;

use super::ArchivedId;
use crate::rkyv_util::id::IdRkyv;

/// Used to archive `Option<Id<T>>`, `Vec<Id<T>>`, `&[Id<T>]`,
/// and `Box<[Id<T>]>` more efficiently than [`Map<IdRkyv>`](rkyv::with::Map).
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::id::IdRkyvMap;
/// use twilight_model::id::Id;
///
/// #[derive(Archive)]
/// struct Cached<'a, T> {
///     #[rkyv(with = IdRkyvMap)] // same as `with = MapNiche<IdRkyv, IdRkyv>`
///     id_opt: Option<Id<T>>,
///     #[rkyv(with = IdRkyvMap)]
///     id_vec: Vec<Id<T>>,
///     #[rkyv(with = IdRkyvMap)]
///     id_slice: &'a [Id<T>],
///     #[rkyv(with = IdRkyvMap)]
///     id_box: Box<[Id<T>]>,
/// }
/// ```
pub struct IdRkyvMap;

impl<T> ArchiveWith<Option<Id<T>>> for IdRkyvMap {
    type Archived = <MapNiche<IdRkyv, IdRkyv> as ArchiveWith<Option<Id<T>>>>::Archived;
    type Resolver = <MapNiche<IdRkyv, IdRkyv> as ArchiveWith<Option<Id<T>>>>::Resolver;

    fn resolve_with(id: &Option<Id<T>>, resolver: Self::Resolver, out: Place<Self::Archived>) {
        MapNiche::<IdRkyv, IdRkyv>::resolve_with(id, resolver, out);
    }
}

impl<S: Fallible + ?Sized, T> SerializeWith<Option<Id<T>>, S> for IdRkyvMap {
    fn serialize_with(opt: &Option<Id<T>>, s: &mut S) -> Result<Self::Resolver, S::Error> {
        MapNiche::<IdRkyv, IdRkyv>::serialize_with(opt, s)
    }
}

impl<D: Fallible + ?Sized, T> DeserializeWith<NichedOption<ArchivedId<T>, IdRkyv>, Option<Id<T>>, D>
    for IdRkyvMap
{
    fn deserialize_with(
        opt: &NichedOption<ArchivedId<T>, IdRkyv>,
        d: &mut D,
    ) -> Result<Option<Id<T>>, D::Error> {
        let Some(archived) = opt.as_ref() else {
            return Ok(None);
        };

        IdRkyv::deserialize_with(archived, d).map(Some)
    }
}

/// Auxiliary trait to provide the most efficient (de)serializations of
/// `&[Id<T>]` across every endian.
trait INonZeroU64: Archive<Archived = Self> {
    /// Serialize `&[Id<T>]` while leveraging `NonZeroU64` and
    /// `Archived<NonZeroU64>` sharing the same layout when possible.
    fn serialize<S, T>(
        list: &[Id<T>],
        serializer: &mut S,
    ) -> Result<VecResolver, <S as Fallible>::Error>
    where
        S: Fallible + Allocator + Writer + ?Sized;

    /// Deserialize an archived `Vec<Id<T>>` while leveraging `NonZeroU64` and
    /// `Archived<NonZeroU64>` sharing the same layout when possible.
    fn deserialize<T, D>(
        archived: &ArchivedVec<ArchivedId<T>>,
        deserializer: &mut D,
    ) -> Result<Vec<Id<T>>, D::Error>
    where
        D: Fallible + ?Sized;
}

macro_rules! impl_non_zero {
    ($ty:path, $endian:literal) => {
        impl INonZeroU64 for $ty {
            fn serialize<S, T>(
                ids: &[Id<T>],
                serializer: &mut S,
            ) -> Result<VecResolver, <S as Fallible>::Error>
            where
                S: Fallible + Allocator + Writer + ?Sized,
            {
                const fn with_ids<T>(ids: &[Id<T>]) -> &[With<Id<T>, IdRkyv>] {
                    let ptr = ptr::from_ref(ids) as *const [With<Id<T>, IdRkyv>];

                    // SAFETY: `With` is just a transparent wrapper
                    unsafe { &*ptr }
                }

                if cfg!(target_endian = $endian) {
                    let pos =
                        serializer.align_for::<<With<Id<T>, IdRkyv> as Archive>::Archived>()?;

                    // SAFETY: `NonZeroU64` and `Archived<NonZeroU64>` share
                    // the same layout.
                    let as_bytes = unsafe {
                        slice::from_raw_parts(ids.as_ptr().cast::<u8>(), mem::size_of_val(ids))
                    };

                    serializer.write(as_bytes)?;

                    Ok(VecResolver::from_pos(pos))
                } else {
                    ArchivedVec::serialize_from_slice(with_ids(ids), serializer)
                }
            }

            fn deserialize<T, D>(
                archived: &ArchivedVec<ArchivedId<T>>,
                deserializer: &mut D,
            ) -> Result<Vec<Id<T>>, D::Error>
            where
                D: Fallible + ?Sized,
            {
                if cfg!(target_endian = $endian) {
                    // SAFETY: `NonZeroU64` and `Archived<NonZeroU64>` share
                    // the same layout.
                    let slice = unsafe { &*(ptr::from_ref(archived.as_slice()) as *const [Id<T>]) };

                    Ok(slice.to_owned())
                } else {
                    With::<_, Map<IdRkyv>>::cast(archived).deserialize(deserializer)
                }
            }
        }
    };
}

impl_non_zero!(rkyv::rend::NonZeroU64_le, "little");
impl_non_zero!(rkyv::rend::NonZeroU64_be, "big");

// Vec<Id<T>>

impl<T> ArchiveWith<Vec<Id<T>>> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    fn resolve_with(ids: &Vec<Id<T>>, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_len(ids.len(), resolver, out);
    }
}

impl<S, T> SerializeWith<Vec<Id<T>>, S> for IdRkyvMap
where
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        ids: &Vec<Id<T>>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::serialize(ids, serializer)
    }
}

impl<D: Fallible + ?Sized, T> DeserializeWith<ArchivedVec<ArchivedId<T>>, Vec<Id<T>>, D>
    for IdRkyvMap
{
    fn deserialize_with(
        archived: &ArchivedVec<ArchivedId<T>>,
        deserializer: &mut D,
    ) -> Result<Vec<Id<T>>, <D as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::deserialize(archived, deserializer)
    }
}

// &[Id<T>]

impl<T> ArchiveWith<&[Id<T>]> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    fn resolve_with(ids: &&[Id<T>], resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_len(ids.len(), resolver, out);
    }
}

impl<S, T> SerializeWith<&[Id<T>], S> for IdRkyvMap
where
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        ids: &&[Id<T>],
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::serialize(ids, serializer)
    }
}

// [Id<T>]

impl<T> ArchiveWith<[Id<T>]> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    fn resolve_with(ids: &[Id<T>], resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_len(ids.len(), resolver, out);
    }
}

impl<S, T> SerializeWith<[Id<T>], S> for IdRkyvMap
where
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        ids: &[Id<T>],
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::serialize(ids, serializer)
    }
}

// Box<[Id<T>]>

impl<T> ArchiveWith<Box<[Id<T>]>> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    fn resolve_with(ids: &Box<[Id<T>]>, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_len(ids.len(), resolver, out);
    }
}

impl<S, T> SerializeWith<Box<[Id<T>]>, S> for IdRkyvMap
where
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        ids: &Box<[Id<T>]>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::serialize(ids, serializer)
    }
}

impl<D: Fallible + ?Sized, T> DeserializeWith<ArchivedVec<ArchivedId<T>>, Box<[Id<T>]>, D>
    for IdRkyvMap
{
    fn deserialize_with(
        archived: &ArchivedVec<ArchivedId<T>>,
        deserializer: &mut D,
    ) -> Result<Box<[Id<T>]>, <D as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::deserialize(archived, deserializer)
            .map(Vec::into_boxed_slice)
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_id_map() -> Result<(), Error> {
        let ids = [Some(Id::new(123)), None];

        for id in ids {
            let bytes = rkyv::to_bytes(With::<_, IdRkyvMap>::cast(&id))?;

            #[cfg(not(feature = "bytecheck"))]
            let archived: &NichedOption<ArchivedId<()>, IdRkyv> =
                unsafe { rkyv::access_unchecked(&bytes) };

            #[cfg(feature = "bytecheck")]
            let archived: &NichedOption<ArchivedId<()>, IdRkyv> = rkyv::access(&bytes)?;

            assert_eq!(id, archived.as_ref().copied().map(Id::from));

            let deserialized: Option<Id<()>> =
                rkyv::deserialize(With::<_, IdRkyvMap>::cast(archived))?;

            assert_eq!(id, deserialized);
        }

        Ok(())
    }

    #[test]
    fn test_rkyv_id_vec() -> Result<(), Error> {
        let ids = vec![Id::new(123), Id::new(234)];
        let bytes = rkyv::to_bytes(With::<_, IdRkyvMap>::cast(&ids))?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &ArchivedVec<ArchivedId<()>> = unsafe { rkyv::access_unchecked(&bytes) };

        #[cfg(feature = "bytecheck")]
        let archived: &ArchivedVec<ArchivedId<()>> = rkyv::access(&bytes)?;

        for (archived, id) in archived.iter().zip(ids.iter()) {
            assert_eq!(archived.get(), id.get());
        }

        let deserialized: Vec<Id<()>> = rkyv::deserialize(With::<_, IdRkyvMap>::cast(archived))?;

        assert_eq!(ids, deserialized);

        Ok(())
    }
}
