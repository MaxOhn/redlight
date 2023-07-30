use std::error::Error as StdError;
use thiserror::Error as ThisError;

/// Represents all the ways something can fail.
#[derive(Debug, ThisError)]
pub enum CacheError {
    #[cfg(feature = "bb8")]
    #[error("failed to create redis pool")]
    CreatePool(#[source] crate::redis::RedisError),
    #[cfg(feature = "bb8")]
    #[error("failed to get a connection")]
    GetConnection(#[source] bb8_redis::bb8::RunError<crate::redis::RedisError>),

    #[cfg(feature = "deadpool")]
    #[error("failed to create redis pool")]
    CreatePool(#[from] deadpool_redis::CreatePoolError),
    #[cfg(feature = "deadpool")]
    #[error("failed to get a connection")]
    GetConnection(#[source] deadpool_redis::PoolError),

    #[error("received invalid response from redis")]
    InvalidResponse,
    #[error("redis error")]
    Redis(#[from] crate::redis::RedisError),
    #[error("cached bytes did not correspond to the cached type")]
    Validation(#[source] Box<dyn StdError>),
}
