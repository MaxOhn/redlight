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

    #[cfg(feature = "validation")]
    #[error("cached bytes did not correspond to the cached type")]
    Validation(#[source] Box<dyn StdError>),

    #[error("received invalid response from redis")]
    InvalidResponse,
    #[error("redis error")]
    Redis(#[from] crate::redis::RedisError),
    #[error(transparent)]
    Serialization(#[from] SerializeError),
}

#[derive(Debug, ThisError)]
pub enum SerializeError {
    #[error("failed to serialize channel")]
    Channel(#[source] Box<dyn StdError>),
    #[error("failed to serialize current user")]
    CurrentUser(#[source] Box<dyn StdError>),
    #[error("failed to serialize emoji")]
    Emoji(#[source] Box<dyn StdError>),
    #[error("failed to serialize guild")]
    Guild(#[source] Box<dyn StdError>),
    #[error("failed to serialize integration")]
    Integration(#[source] Box<dyn StdError>),
    #[error("failed to serialize member")]
    Member(#[source] Box<dyn StdError>),
    #[error("failed to serialize message")]
    Message(#[source] Box<dyn StdError>),
    #[error("failed to serialize presence")]
    Presence(#[source] Box<dyn StdError>),
    #[error("failed to serialize role")]
    Role(#[source] Box<dyn StdError>),
    #[error("failed to serialize stage instance")]
    StageInstance(#[source] Box<dyn StdError>),
    #[error("failed to serialize sticker")]
    Sticker(#[source] Box<dyn StdError>),
    #[error("failed to serialize user")]
    User(#[source] Box<dyn StdError>),
    #[error("failed to serialize voice state")]
    VoiceState(#[source] Box<dyn StdError>),
}
