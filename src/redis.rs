#[cfg(feature = "bb8")]
pub(crate) use bb8::*;
#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
pub(crate) use deadpool::*;
use tracing::trace;

use crate::{CacheResult, RedisCache};

#[cfg(feature = "bb8")]
mod bb8 {
    pub use bb8_redis::{
        bb8::{PooledConnection, RunError},
        redis::*,
        RedisConnectionManager,
    };

    pub type Pool = bb8_redis::bb8::Pool<RedisConnectionManager>;

    pub struct Connection<'a>(pub(super) PooledConnection<'a, RedisConnectionManager>);

    impl<'a> Connection<'a> {
        pub async fn get(pool: &'a Pool) -> Result<Connection<'a>, RunError<RedisError>> {
            pool.get().await.map(Self)
        }
    }

    pub struct DedicatedConnection(
        pub(super) <RedisConnectionManager as bb8_redis::bb8::ManageConnection>::Connection,
    );

    impl DedicatedConnection {
        pub async fn get(pool: &Pool) -> Result<Self, RedisError> {
            pool.dedicated_connection().await.map(Self)
        }
    }
}

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
mod deadpool {
    use std::marker::PhantomData;

    pub use deadpool_redis::{redis::*, Pool};
    use deadpool_redis::{Connection as DeadpoolConnection, PoolError};

    type InnerConnection = deadpool_redis::Connection;

    pub struct Connection<'a>(
        pub(super) InnerConnection,
        // not necessary but makes handling between bb8 and deadpool easier
        PhantomData<&'a ()>,
    );

    impl<'a> Connection<'a> {
        const fn new(inner: InnerConnection) -> Self {
            Self(inner, PhantomData)
        }

        pub async fn get(pool: &'a Pool) -> Result<Connection<'a>, PoolError> {
            pool.get().await.map(Self::new)
        }
    }

    pub struct DedicatedConnection(pub(super) aio::Connection);

    impl DedicatedConnection {
        pub async fn get(pool: &Pool) -> Result<Self, PoolError> {
            pool.get().await.map(DeadpoolConnection::take).map(Self)
        }
    }
}

impl aio::ConnectionLike for Connection<'_> {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        aio::ConnectionLike::req_packed_command(&mut *self.0, cmd)
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        aio::ConnectionLike::req_packed_commands(&mut *self.0, cmd, offset, count)
    }

    fn get_db(&self) -> i64 {
        aio::ConnectionLike::get_db(&*self.0)
    }
}

impl DedicatedConnection {
    pub(crate) fn into_pubsub(self) -> aio::PubSub {
        self.0.into_pubsub()
    }
}

impl aio::ConnectionLike for DedicatedConnection {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        aio::ConnectionLike::req_packed_command(&mut self.0, cmd)
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        aio::ConnectionLike::req_packed_commands(&mut self.0, cmd, offset, count)
    }

    fn get_db(&self) -> i64 {
        aio::ConnectionLike::get_db(&self.0)
    }
}

pub(crate) enum ConnectionState<'c, C> {
    Cache(&'c RedisCache<C>),
    Connection(Connection<'c>),
}

impl<'c, C> ConnectionState<'c, C> {
    pub(crate) const fn new(cache: &'c RedisCache<C>) -> Self {
        Self::Cache(cache)
    }

    pub(crate) async fn get(&mut self) -> CacheResult<&mut Connection<'c>> {
        match self {
            ConnectionState::Cache(cache) => {
                trace!(conn_ready = false);

                let conn = cache.connection().await?;
                *self = Self::Connection(conn);

                let Self::Connection(conn) = self else {
                    // SAFETY: we just assigned a connection
                    unsafe { std::hint::unreachable_unchecked() }
                };

                Ok(conn)
            }
            ConnectionState::Connection(conn) => {
                trace!(conn_ready = true);

                Ok(conn)
            }
        }
    }
}
