#[cfg(feature = "bb8")]
pub(crate) use bb8::*;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
pub(crate) use deadpool::*;

#[cfg(feature = "bb8")]
mod bb8 {
    use bb8_redis::bb8::ManageConnection;
    pub use bb8_redis::{
        bb8::{PooledConnection, RunError},
        redis::*,
        RedisConnectionManager,
    };

    pub type Pool = bb8_redis::bb8::Pool<RedisConnectionManager>;

    type InnerConnection = <RedisConnectionManager as ManageConnection>::Connection;

    pub struct Connection<'a>(PooledConnection<'a, RedisConnectionManager>);

    impl<'a> Connection<'a> {
        pub async fn get(pool: &'a Pool) -> Result<Connection<'a>, RunError<RedisError>> {
            pool.get().await.map(Self)
        }

        pub(super) fn inner(&self) -> &InnerConnection {
            &self.0
        }

        pub(super) fn inner_mut(&mut self) -> &mut InnerConnection {
            &mut self.0
        }
    }
}

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
mod deadpool {
    use std::marker::PhantomData;

    use deadpool_redis::PoolError;
    pub use deadpool_redis::{redis::*, Pool};

    type InnerConnection = deadpool_redis::Connection;

    pub struct Connection<'a> {
        inner: InnerConnection,
        // not necessary but makes handling between bb8 and deadpool easier
        _lifetime: PhantomData<&'a ()>,
    }

    impl<'a> Connection<'a> {
        pub async fn get(pool: &'a Pool) -> Result<Connection<'a>, PoolError> {
            pool.get().await.map(|inner| Self {
                inner,
                _lifetime: PhantomData,
            })
        }

        pub(super) fn inner(&self) -> &InnerConnection {
            &self.inner
        }

        pub(super) fn inner_mut(&mut self) -> &mut InnerConnection {
            &mut self.inner
        }
    }
}

impl aio::ConnectionLike for Connection<'_> {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        aio::ConnectionLike::req_packed_command(self.inner_mut(), cmd)
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        aio::ConnectionLike::req_packed_commands(self.inner_mut(), cmd, offset, count)
    }

    fn get_db(&self) -> i64 {
        aio::ConnectionLike::get_db(self.inner())
    }
}
