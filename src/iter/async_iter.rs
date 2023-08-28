use std::{
    future::Future,
    marker::PhantomData,
    mem,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
    vec::IntoIter,
};

use futures_util::{stream::StreamExt, Stream};
use itoa::Buffer;
use pin_project::pin_project;

use crate::{
    config::Cacheable,
    redis::{
        aio::ConnectionLike, Cmd, Connection, FromRedisValue, RedisFuture, RedisResult, Value,
    },
    CacheError, CacheResult, CachedArchive,
};

/// An iterator that fetches cached entries asynchronously.
///
/// The items are of type [`CachedArchive`] wrapped in a [`Result`].
#[pin_project(project = AsyncIterProj)]
pub struct AsyncIter<'c, T> {
    ids: IntoIter<u64>,
    itoa_buf: Buffer,
    key_prefix_len: usize,
    key_buf: Vec<u8>,
    next: Next,
    #[pin]
    data: Box<StaticData<'c>>,
    _phantom: PhantomData<T>,
}

impl<'c, T: Cacheable> AsyncIter<'c, T> {
    pub(crate) fn new(conn: Connection<'c>, ids: Vec<u64>, key_prefix: Vec<u8>) -> Self {
        Self::new_with_buf(conn, ids, key_prefix, Buffer::new())
    }

    pub(crate) fn new_with_buf(
        conn: Connection<'c>,
        ids: Vec<u64>,
        key_prefix: Vec<u8>,
        itoa_buf: Buffer,
    ) -> Self {
        Self {
            ids: ids.into_iter(),
            itoa_buf,
            key_prefix_len: key_prefix.len(),
            key_buf: key_prefix,
            next: Next::Create,
            data: Box::new(StaticData::new(conn)),
            _phantom: PhantomData,
        }
    }

    /// Retrieve the next item from the cache.
    pub async fn next_item(&mut self) -> Option<CacheResult<CachedArchive<T>>> {
        self.next().await
    }

    fn next_fut(
        ids: &mut IntoIter<u64>,
        itoa_buf: &mut Buffer,
        key_prefix_len: usize,
        key_buf: &mut Vec<u8>,
        mut data: Pin<&mut Box<StaticData<'_>>>,
    ) -> Option<RedisFuture<'static, Value>> {
        // SAFETY:
        // The original `Cmd` and `Connection` come from `StaticData`
        // which is boxed, ensuring that fields won't move.
        // We also know that the resulting future lives at most as long as that
        // Box so it is fine for us to consider the lifetime as static.
        fn extend_cmd_lifetime(cmd: &Cmd) -> &'static Cmd {
            unsafe { &*(cmd as *const _) }
        }

        fn extend_conn_lifetime(conn: &mut Connection<'_>) -> &'static mut Connection<'static> {
            unsafe { &mut *(conn as *mut Connection<'_>).cast::<Connection<'static>>() }
        }

        let id = ids.next()?;

        key_buf.truncate(key_prefix_len);
        let id = itoa_buf.format(id);
        key_buf.extend_from_slice(id.as_bytes());
        let cmd = Cmd::get(key_buf.as_slice());

        let cmd = data.cmd.write(cmd);
        let cmd = extend_cmd_lifetime(cmd);

        let conn = extend_conn_lifetime(&mut data.conn);

        Some(conn.req_packed_command(cmd))
    }
}

impl<'c, T: Cacheable> Stream for AsyncIter<'c, T> {
    type Item = CacheResult<CachedArchive<T>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let AsyncIterProj {
            ids,
            itoa_buf,
            key_prefix_len: len,
            key_buf,
            next,
            mut data,
            _phantom,
        } = self.project();

        loop {
            match next {
                Next::Create => match Self::next_fut(ids, itoa_buf, *len, key_buf, data.as_mut()) {
                    Some(fut) => *next = Next::InFlight(fut),
                    None => {
                        *next = Next::Completed;

                        return Poll::Ready(None);
                    }
                },
                Next::InFlight(fut) => match Pin::new(fut).poll(cx) {
                    Poll::Ready(res) => *next = Next::Ready(res),
                    Poll::Pending => return Poll::Pending,
                },
                Next::Ready(res) => {
                    let res = mem::replace(res, Ok(Value::Nil));
                    *next = Next::Create;

                    match res.and_then(|value| Option::<Vec<u8>>::from_redis_value(&value)) {
                        Ok(Some(bytes)) => {
                            let bytes = bytes.into_boxed_slice();

                            #[cfg(feature = "validation")]
                            let archived_res = CachedArchive::new(bytes);

                            #[cfg(not(feature = "validation"))]
                            let archived_res = Ok(CachedArchive::new_unchecked(bytes));

                            return Poll::Ready(Some(archived_res));
                        }
                        Ok(None) => {}
                        Err(err) => return Poll::Ready(Some(Err(CacheError::Redis(err)))),
                    }
                }
                Next::Completed => panic!("poll after future completed"),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, max) = self.ids.size_hint();

        (0, max)
    }
}

enum Next {
    Create,
    InFlight(RedisFuture<'static, Value>),
    Ready(RedisResult<Value>),
    Completed,
}

// It will be crucial for this data to not move during future polling
// so this should be boxed.
struct StaticData<'c> {
    conn: Connection<'c>,
    cmd: MaybeUninit<Cmd>,
}

impl<'c> StaticData<'c> {
    fn new(conn: Connection<'c>) -> Self {
        Self {
            conn,
            cmd: MaybeUninit::uninit(),
        }
    }
}
