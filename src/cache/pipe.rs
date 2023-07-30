use std::ops::DerefMut;

use crate::{
    key::RedisKey,
    redis::{FromRedisValue, Pipeline, ToRedisArgs},
    CacheResult, RedisCache,
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

    pub(crate) fn mset<V: ToRedisArgs>(&mut self, items: &[(RedisKey, V)]) -> &mut Self {
        self.pipe.mset(items);

        self
    }

    pub(crate) fn sadd(&mut self, key: RedisKey, member: impl ToRedisArgs) -> &mut Self {
        self.pipe.sadd(key, member);

        self
    }

    pub(crate) fn scard(&mut self, key: RedisKey) {
        self.pipe.scard(key);
    }

    pub(crate) fn set(&mut self, key: RedisKey, bytes: &[u8]) -> &mut Self {
        self.pipe.set(key, bytes);

        self
    }

    pub(crate) fn set_ex(&mut self, key: RedisKey, bytes: &[u8], seconds: usize) -> &mut Self {
        self.pipe.set_ex(key, bytes, seconds);

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