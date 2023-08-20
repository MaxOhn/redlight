use std::ops::{Deref, DerefMut};

#[cfg(feature = "bb8")]
pub(crate) use bb8::*;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
pub(crate) use deadpool::*;

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

    #[cfg(feature = "metrics")]
    pub struct DedicatedConnection(
        pub(super) <RedisConnectionManager as bb8_redis::bb8::ManageConnection>::Connection,
    );

    #[cfg(feature = "metrics")]
    impl DedicatedConnection {
        pub async fn get(pool: &Pool) -> Result<Self, RedisError> {
            pool.dedicated_connection().await.map(Self)
        }
    }
}

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
mod deadpool {
    use std::marker::PhantomData;

    use deadpool_redis::PoolError;
    pub use deadpool_redis::{redis::*, Pool};

    type InnerConnection = deadpool_redis::Connection;

    pub struct Connection<'a>(
        pub(super) InnerConnection,
        // not necessary but makes handling between bb8 and deadpool easier
        PhantomData<&'a ()>,
    );

    impl<'a> Connection<'a> {
        fn new(inner: InnerConnection) -> Self {
            Self(inner, PhantomData)
        }

        pub async fn get(pool: &'a Pool) -> Result<Connection<'a>, PoolError> {
            pool.get().await.map(Self::new)
        }
    }

    #[cfg(feature = "metrics")]
    pub struct DedicatedConnection(pub(super) InnerConnection);

    #[cfg(feature = "metrics")]
    impl DedicatedConnection {
        pub async fn get(pool: &Pool) -> Result<Self, PoolError> {
            pool.get().await.map(Self)
        }
    }
}

impl aio::ConnectionLike for Connection<'_> {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        aio::ConnectionLike::req_packed_command(self.0.deref_mut(), cmd)
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        aio::ConnectionLike::req_packed_commands(self.0.deref_mut(), cmd, offset, count)
    }

    fn get_db(&self) -> i64 {
        aio::ConnectionLike::get_db(self.0.deref())
    }
}

#[cfg(feature = "metrics")]
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
