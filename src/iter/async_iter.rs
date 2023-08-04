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
        loop {
            let id = self.ids.next()?;

            self.key_buf.truncate(self.key_prefix_len);
            let id = self.itoa_buf.format(id);
            self.key_buf.extend_from_slice(id.as_bytes());

            let key = self.key_buf.as_slice();

            let res = match self.conn.get::<_, Option<Vec<u8>>>(key).await {
                #[cfg(feature = "validation")]
                Ok(Some(bytes)) => CachedArchive::new(bytes.into_boxed_slice()),
                #[cfg(not(feature = "validation"))]
                Ok(Some(bytes)) => Ok(CachedArchive::new_unchecked(bytes.into_boxed_slice())),
                Ok(None) => continue,
                Err(err) => Err(CacheError::Redis(err)),
            };

            return Some(res);
        }
    }

    // TODO: implement .nth, .skip, ... efficiently
}
