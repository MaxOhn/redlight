use std::{iter::FusedIterator, marker::PhantomData, vec::IntoIter};

use rkyv::util::AlignedVec;

use crate::{config::CheckedArchived, util::BytesWrap, CachedArchive};

type Bytes = BytesWrap<AlignedVec<16>>;

#[cfg(feature = "bytecheck")]
pub type EntryResult<T> = Result<CachedArchive<T>, crate::error::ValidationError>;

#[cfg(not(feature = "bytecheck"))]
pub type EntryResult<T> = CachedArchive<T>;

/// Iterator over [`Option<EntryResult<T>>`].
///
/// Returns `None` if an entry was not found.
#[derive(Clone, Debug)]
pub struct OptionalCacheIter<T> {
    inner: IntoIter<Bytes>,
    _item: PhantomData<T>,
}

impl<T: CheckedArchived> OptionalCacheIter<T> {
    pub(crate) fn new(bytes: Vec<Bytes>) -> Self {
        Self {
            inner: bytes.into_iter(),
            _item: PhantomData,
        }
    }
}

impl<T: CheckedArchived> Iterator for OptionalCacheIter<T> {
    type Item = Option<EntryResult<T>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(process_bytes)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.inner.fold(init, fold(f))
    }
}

fn process_bytes<T: CheckedArchived>(BytesWrap(bytes): Bytes) -> Option<EntryResult<T>> {
    if bytes.is_empty() {
        return None;
    }

    #[cfg(feature = "bytecheck")]
    let archived = CachedArchive::new(bytes);

    #[cfg(not(feature = "bytecheck"))]
    let archived = CachedArchive::new_unchecked(bytes);

    Some(archived)
}

fn fold<T: CheckedArchived, B>(
    mut f: impl FnMut(B, Option<EntryResult<T>>) -> B,
) -> impl FnMut(B, Bytes) -> B {
    move |acc, elt| f(acc, process_bytes(elt))
}

impl<T: CheckedArchived> DoubleEndedIterator for OptionalCacheIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(process_bytes)
    }

    #[inline]
    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.inner.rfold(init, fold(f))
    }
}

impl<T: CheckedArchived> ExactSizeIterator for OptionalCacheIter<T> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T: CheckedArchived> FusedIterator for OptionalCacheIter<T> {}

/// Iterator over [`EntryResult<T>`].
#[derive(Clone, Debug)]
pub struct CacheIter<T> {
    inner: OptionalCacheIter<T>,
}

impl<T> CacheIter<T> {
    pub(crate) const fn new(inner: OptionalCacheIter<T>) -> Self {
        Self { inner }
    }
}

impl<T: CheckedArchived> Iterator for CacheIter<T> {
    type Item = EntryResult<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        process_next(&mut self.inner, OptionalCacheIter::next)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.inner.len()))
    }
}

fn process_next<T: CheckedArchived>(
    iter: &mut OptionalCacheIter<T>,
    mut f: impl FnMut(&mut OptionalCacheIter<T>) -> Option<Option<EntryResult<T>>>,
) -> Option<EntryResult<T>> {
    loop {
        match f(iter) {
            Some(entry @ Some(_)) => return entry,
            Some(None) => {}
            None => return None,
        }
    }
}

impl<T: CheckedArchived> DoubleEndedIterator for CacheIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        process_next(&mut self.inner, OptionalCacheIter::next_back)
    }
}

impl<T: CheckedArchived> FusedIterator for CacheIter<T> {}
