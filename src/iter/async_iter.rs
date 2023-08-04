use std::{marker::PhantomData, vec::IntoIter};

use itoa::Buffer;

use crate::{
    config::Cacheable,
    redis::{AsyncCommands, Connection},
    CacheError, CacheResult, CachedArchive,
};

pub struct AsyncIter<'c, T> {
    conn: Connection<'c>,
    ids: IntoIter<u64>,
    phantom: PhantomData<T>,
    itoa_buf: Buffer,
    key_prefix_len: usize,
    key_buf: Vec<u8>,
}

impl<'c, T: Cacheable> AsyncIter<'c, T> {
    pub(crate) fn new(conn: Connection<'c>, ids: Vec<u64>, key_prefix: &[u8]) -> Self {
        Self {
            conn,
            ids: ids.into_iter(),
            phantom: PhantomData,
            itoa_buf: Buffer::new(),
            key_prefix_len: key_prefix.len(),
            key_buf: key_prefix.to_owned(),
        }
    }

    pub async fn next_item(&mut self) -> Option<CacheResult<CachedArchive<T>>> {
        let id = self.ids.next()?;

        self.key_buf.truncate(self.key_prefix_len);
        let id = self.itoa_buf.format(id);
        self.key_buf.extend_from_slice(id.as_bytes());

        let res = match self.conn.get::<_, Vec<u8>>(self.key_buf.as_slice()).await {
            #[cfg(feature = "validation")]
            Ok(bytes) => CachedArchive::new(bytes.into_boxed_slice()),
            #[cfg(not(feature = "validation"))]
            Ok(bytes) => Ok(CachedArchive::new_unchecked(bytes.into_boxed_slice())),
            Err(err) => Err(CacheError::Redis(err)),
        };

        Some(res)
    }

    // TODO: implement .nth, .skip, ... efficiently
}
