use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    num::NonZeroU64,
    ptr::addr_of_mut,
};

use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Fallible,
};
use twilight_model::id::Id;

/// Used to archive `Option<Id<T>>` more efficiently than with [`Map<IdRkyv>`](rkyv::with::Map).
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use twilight_model::id::Id;
/// use twilight_redis::rkyv_util::id::IdRkyvMap;
///
/// #[derive(Archive)]
/// struct Cached<T> {
///     #[with(IdRkyvMap)]
///     id_opt: Option<Id<T>>,
/// }
/// ```
pub struct IdRkyvMap;

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

    /// Resolves an `ArchivedIdOption` from an `Option<Id<T>>`.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    pub unsafe fn resolve_from_id<T>(opt: Option<Id<T>>, out: *mut Self) {
        let fo = addr_of_mut!((*out).inner);

        if let Some(id) = opt {
            fo.write(id.get());
        } else {
            fo.write(0);
        }
    }
}

impl Debug for ArchivedIdOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.to_nonzero_option(), f)
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
