use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    marker::PhantomData,
    num::NonZeroU64,
    ptr::{addr_of, addr_of_mut},
};

use rkyv::{
    ser::{ScratchSpace, Serializer},
    vec::{ArchivedVec, VecResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith, With},
    Archived, Deserialize, Fallible,
};
use twilight_model::id::Id;

use crate::rkyv_util::id::IdRkyv;

use super::ArchivedId;

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

/// An efficiently archived `Option<Id<T>>`.
pub struct ArchivedIdOption<T> {
    inner: Archived<u64>,
    phantom: PhantomData<fn(T) -> T>,
}

impl<T> Clone for ArchivedIdOption<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArchivedIdOption<T> {}

impl<T> PartialEq for ArchivedIdOption<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for ArchivedIdOption<T> {}

impl<T> PartialEq<Option<Id<T>>> for ArchivedIdOption<T> {
    fn eq(&self, other: &Option<Id<T>>) -> bool {
        self.to_id_option() == *other
    }
}

impl<T> ArchivedIdOption<T> {
    /// Convert into an `Option<NonZeroU64>`.
    pub fn to_nonzero_option(self) -> Option<NonZeroU64> {
        #[allow(clippy::if_not_else)]
        if self.inner != 0 {
            // SAFETY: NonZero types have the same memory layout and bit patterns as
            // their integer counterparts, regardless of endianness
            let as_nonzero = unsafe { *(addr_of!(self.inner).cast::<Archived<NonZeroU64>>()) };

            // the .into() is necessary in case the `archive_le` or `archive_be`
            // features are enabled in rkyv
            #[allow(clippy::useless_conversion)]
            Some(as_nonzero.into())
        } else {
            None
        }
    }

    /// Convert into an `Option<Id<T>>`.
    pub fn to_id_option(self) -> Option<Id<T>> {
        self.to_nonzero_option().map(Id::from)
    }

    /// Resolves an `ArchivedIdOption` from an `Option<Id<T>>`.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    #[allow(clippy::similar_names)]
    pub unsafe fn resolve_from_id(opt: Option<Id<T>>, out: *mut Self) {
        let fo = addr_of_mut!((*out).inner);
        let id = opt.map_or(0, Id::get);

        // the .into() is necessary in case the `archive_le` or `archive_be`
        // features are enabled in rkyv
        #[allow(clippy::useless_conversion)]
        fo.write(id.into());
    }
}

impl<T> Debug for ArchivedIdOption<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.to_nonzero_option(), f)
    }
}

#[cfg(feature = "validation")]
#[cfg_attr(docsrs, doc(cfg(feature = "validation")))]
const _: () = {
    use std::convert::Infallible;

    use rkyv::CheckBytes;

    impl<C: ?Sized, T> CheckBytes<C> for ArchivedIdOption<T> {
        type Error = Infallible;

        unsafe fn check_bytes<'a>(value: *const Self, _: &mut C) -> Result<&'a Self, Self::Error> {
            Ok(&*value)
        }
    }
};

impl<T> ArchiveWith<Option<Id<T>>> for IdRkyvMap {
    type Archived = ArchivedIdOption<T>;
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

impl<D: Fallible + ?Sized, T> DeserializeWith<ArchivedIdOption<T>, Option<Id<T>>, D> for IdRkyvMap {
    #[inline]
    fn deserialize_with(
        archived: &ArchivedIdOption<T>,
        deserializer: &mut D,
    ) -> Result<Option<Id<T>>, D::Error> {
        archived.deserialize(deserializer)
    }
}

impl<D: Fallible + ?Sized, T> Deserialize<Option<Id<T>>, D> for ArchivedIdOption<T> {
    fn deserialize(&self, _: &mut D) -> Result<Option<Id<T>>, <D as Fallible>::Error> {
        Ok(self.to_id_option())
    }
}

/// Auxiliary trait to provide the most efficient (de)serializations of `&[Id<T>]` across every endian.
trait INonZeroU64 {
    /// Serialize `&[Id<T>]` assuming that `Self` is `Archived<NonZeroU64>`
    /// or at least has the same layout.
    fn serialize<S, T>(
        list: &[Id<T>],
        serializer: &mut S,
    ) -> Result<VecResolver, <S as Fallible>::Error>
    where
        S: Fallible + Serializer + ScratchSpace + ?Sized;

    /// Deserialize an archived `Vec<Id<T>>` assuming that `Self` is `Archived<NonZeroU64>`
    /// or at least has the same layout.
    fn deserialize<T>(archived: &ArchivedVec<ArchivedId<T>>) -> Vec<Id<T>>;
}

impl INonZeroU64 for NonZeroU64 {
    fn serialize<S, T>(
        ids: &[Id<T>],
        serializer: &mut S,
    ) -> Result<VecResolver, <S as Fallible>::Error>
    where
        S: Fallible + Serializer + ScratchSpace + ?Sized,
    {
        fn wrap_ids<T>(ids: &[Id<T>]) -> &[With<Id<T>, IdRkyv>] {
            let ptr = ids as *const [Id<T>] as *const [With<Id<T>, IdRkyv>];

            // SAFETY: `With` is just a transparent wrapper
            unsafe { &*ptr }
        }

        // SAFETY: The caller guarantees that `NonZeroU64` and
        // `Archived<NonZeroU64>` share the same layout.
        unsafe { ArchivedVec::serialize_copy_from_slice(wrap_ids(ids), serializer) }
    }

    fn deserialize<T>(archived: &ArchivedVec<ArchivedId<T>>) -> Vec<Id<T>> {
        /// # Safety
        ///
        /// It must hold that `NonZeroU64` and `Archived<NonZerou64>` share the same layout.
        unsafe fn cast_archived<T>(ids: &[ArchivedId<T>]) -> &[Id<T>] {
            &*(ids as *const [ArchivedId<T>] as *const [Id<T>])
        }

        // SAFETY: The caller guarantees that `NonZeroU64` and
        // `Archived<NonZeroU64>` share the same layout
        unsafe { cast_archived(archived.as_slice()) }.to_owned()
    }
}

// The only way for us to know whether `rkyv::rend` is available is if our
// `validation` feature is enabled which enables `rkyv/validation` which
// in turn enables `rkyv/rend`.
// FIXME: For the edge case that `validation` is not enabled yet `rkyv/archive_*`
// *is* enabled, our build will currently fail.
#[cfg(feature = "validation")]
const _: () = {
    macro_rules! impl_endian {
        ( $endian:ident: $target_endian:literal ) => {
            impl INonZeroU64 for ::rkyv::rend::$endian<NonZeroU64> {
                fn serialize<S, T>(
                    list: &[Id<T>],
                    serializer: &mut S,
                ) -> Result<VecResolver, <S as Fallible>::Error>
                where
                    S: Fallible + Serializer + ScratchSpace + ?Sized,
                {
                    if cfg!(target_endian = $target_endian) {
                        return <NonZeroU64 as INonZeroU64>::serialize(list, serializer);
                    }

                    #[allow(clippy::items_after_statements)]
                    type Wrapper<T> = With<Id<T>, IdRkyv>;
                    let iter = list.iter().map(Wrapper::<T>::cast);

                    ArchivedVec::serialize_from_iter::<Wrapper<T>, _, _, _>(iter, serializer)
                }

                fn deserialize<T>(archived: &ArchivedVec<ArchivedId<T>>) -> Vec<Id<T>> {
                    if cfg!(target_endian = $target_endian) {
                        <NonZeroU64 as INonZeroU64>::deserialize(archived)
                    } else {
                        archived.iter().copied().map(Id::from).collect()
                    }
                }
            }
        };
    }

    impl_endian!(BigEndian: "big");
    impl_endian!(LittleEndian: "little");
};

// Vec<Id<T>>

impl<T> ArchiveWith<Vec<Id<T>>> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        ids: &Vec<Id<T>>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(ids.len(), pos, resolver, out);
    }
}

impl<S, T> SerializeWith<Vec<Id<T>>, S> for IdRkyvMap
where
    S: Fallible + Serializer + ScratchSpace + ?Sized,
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
        _: &mut D,
    ) -> Result<Vec<Id<T>>, <D as Fallible>::Error> {
        Ok(<Archived<NonZeroU64> as INonZeroU64>::deserialize(archived))
    }
}

// &[Id<T>]

impl<T> ArchiveWith<&[Id<T>]> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        ids: &&[Id<T>],
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(ids.len(), pos, resolver, out);
    }
}

impl<S, T> SerializeWith<&[Id<T>], S> for IdRkyvMap
where
    S: Fallible + Serializer + ScratchSpace + ?Sized,
{
    fn serialize_with(
        ids: &&[Id<T>],
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Archived<NonZeroU64> as INonZeroU64>::serialize(ids, serializer)
    }
}

// Box<[Id<T>]>

impl<T> ArchiveWith<Box<[Id<T>]>> for IdRkyvMap {
    type Archived = ArchivedVec<ArchivedId<T>>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        ids: &Box<[Id<T>]>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(ids.len(), pos, resolver, out);
    }
}

impl<S, T> SerializeWith<Box<[Id<T>]>, S> for IdRkyvMap
where
    S: Fallible + Serializer + ScratchSpace + ?Sized,
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
        _: &mut D,
    ) -> Result<Box<[Id<T>]>, <D as Fallible>::Error> {
        Ok(<Archived<NonZeroU64> as INonZeroU64>::deserialize(archived).into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_id_map() {
        type Wrapper = With<Option<Id<()>>, IdRkyvMap>;

        let ids = [Some(Id::new(123)), None];

        for id in ids {
            let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&id)).unwrap();

            #[cfg(not(feature = "validation"))]
            let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

            #[cfg(feature = "validation")]
            let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

            assert_eq!(id, archived.to_id_option());

            let deserialized: Option<Id<()>> = archived.deserialize(&mut Infallible).unwrap();

            assert_eq!(id, deserialized);
        }
    }

    #[test]
    fn test_rkyv_id_vec() {
        type Wrapper = With<Vec<Id<()>>, IdRkyvMap>;

        let ids = vec![Id::new(123), Id::new(234)];
        let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&ids)).unwrap();

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

        let same_archived = archived
            .iter()
            .zip(ids.iter())
            .all(|(archived, id)| archived.get() == id.get());

        assert!(same_archived);

        let deserialized: Vec<Id<()>> =
            IdRkyvMap::deserialize_with(archived, &mut Infallible).unwrap();

        assert_eq!(ids, deserialized);
    }
}
