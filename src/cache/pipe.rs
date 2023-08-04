use std::ops::DerefMut;

use crate::{
    config::{CacheConfig, Cacheable},
    key::RedisKey,
    redis::{AsyncCommands, FromRedisValue, Pipeline, ToRedisArgs},
    CacheResult, CachedValue, RedisCache,
};

use super::Connection;

pub(crate) struct Pipe<'c, C> {
    cache: &'c RedisCache<C>,
    conn: Option<Connection<'c>>,
    pipe: Pipeline,
}

impl<'c, C> Pipe<'c, C> {
    pub(crate) fn new(cache: &'c RedisCache<C>) -> Self {
        Self {
            cache,
            conn: None,
            pipe: Pipeline::new(),
        }
    }

    pub(crate) async fn query<T: FromRedisValue>(&mut self) -> CacheResult<T> {
        let conn = match self.conn.as_mut() {
            Some(conn) => conn,
            None => self.conn.insert(self.cache.connection().await?),
        };

        let res = self.pipe.query_async(conn.deref_mut()).await?;
        self.pipe.clear();

        Ok(res)
    }

    pub(crate) fn del(&mut self, key: impl ToRedisArgs) -> &mut Self {
        self.pipe.del(key);

        self
    }

    pub(crate) fn ignore(&mut self) {
        self.pipe.ignore();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pipe.cmd_iter().next().is_none()
    }

    pub(crate) fn mset<V: ToRedisArgs>(
        &mut self,
        items: &[(RedisKey, V)],
        expire_seconds: Option<usize>,
    ) -> &mut Self {
        self.pipe.mset(items);

        if let Some(seconds) = expire_seconds {
            for (key, _) in items {
                self.pipe.expire(key, seconds).ignore();
            }
        }

        self
    }

    pub(crate) fn sadd(&mut self, key: RedisKey, member: impl ToRedisArgs) -> &mut Self {
        self.pipe.sadd(key, member);

        self
    }

    pub(crate) fn scard(&mut self, key: RedisKey) {
        self.pipe.scard(key);
    }

    pub(crate) fn set(
        &mut self,
        key: RedisKey,
        bytes: &[u8],
        expire_seconds: Option<usize>,
    ) -> &mut Self {
        if let Some(seconds) = expire_seconds {
            self.pipe.set_ex(key, bytes, seconds);
        } else {
            self.pipe.set(key, bytes);
        }

        self
    }

    pub(crate) fn smembers(&mut self, key: RedisKey) {
        self.pipe.smembers(key);
    }

    pub(crate) fn srem(&mut self, key: RedisKey, member: impl ToRedisArgs) -> &mut Self {
        self.pipe.srem(key, member);

        self
    }
}

impl<'c, C: CacheConfig> Pipe<'c, C> {
    pub(crate) async fn get<T>(&mut self, key: RedisKey) -> CacheResult<Option<CachedValue<T>>>
    where
        T: Cacheable,
    {
        let conn = match self.conn.as_mut() {
            Some(conn) => conn,
            None => self.conn.insert(self.cache.connection().await?),
        };

        let bytes: Vec<u8> = conn.get(key).await?;

        if bytes.is_empty() {
            return Ok(None);
        }

        #[cfg(feature = "validation")]
        let res = CachedValue::new(bytes.into_boxed_slice());

        #[cfg(not(feature = "validation"))]
        let res = Ok(CachedValue::new_unchecked(bytes.into_boxed_slice()));

        res.map(Some)
    }
}
