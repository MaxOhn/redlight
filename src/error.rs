use rkyv::rancor::BoxedError;
use thiserror::Error as ThisError;

use crate::redis::RedisError;

#[cfg(feature = "bb8")]
type DedicatedConnectionError = RedisError;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
type DedicatedConnectionError = deadpool_redis::PoolError;

/// Represents all the ways something can fail.
#[derive(Debug, ThisError)]
pub enum CacheError {
    #[cfg(feature = "bb8")]
    #[error("failed to create redis pool")]
    /// Failed to create redis pool.
    CreatePool(#[source] RedisError),
    #[cfg(feature = "bb8")]
    #[error("failed to get a connection")]
    /// Failed to get a connection.
    GetConnection(#[source] bb8_redis::bb8::RunError<RedisError>),

    #[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
    #[error("failed to create redis pool")]
    /// Failed to create redis pool.
    CreatePool(#[from] deadpool_redis::CreatePoolError),
    #[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
    #[error("failed to get a connection")]
    /// Failed to get a connection.
    GetConnection(#[source] deadpool_redis::PoolError),

    #[cfg(feature = "bytecheck")]
    #[cfg_attr(all(docsrs, not(doctest)), doc(cfg(feature = "bytecheck")))]
    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[cfg(feature = "cold_resume")]
    #[cfg_attr(all(docsrs, not(doctest)), doc(cfg(feature = "cold_resume")))]
    #[error("failed to serialize sessions")]
    /// Failed to serialize sessions.
    SerializeSessions(#[source] BoxedError),

    #[error(transparent)]
    /// Expire-related error.
    Expire(#[from] ExpireError),
    #[error("received invalid response from redis")]
    /// Received invalid response from redis
    InvalidResponse,
    #[error(transparent)]
    /// Meta-related error.
    Meta(#[from] MetaError),
    #[error("redis error")]
    /// Redis error.
    Redis(#[from] RedisError),
    #[error(transparent)]
    /// Serialization-related error.
    Serialization(#[from] SerializeError),
    #[error("failed to update entry")]
    /// Failed to update entry.
    Update(#[from] UpdateError),
}

#[derive(Debug, ThisError)]
#[error("failed to serialize {kind:?}")]
/// Failed to serialize some type.
pub struct SerializeError {
    #[source]
    pub error: BoxedError,
    pub kind: SerializeErrorKind,
}

impl SerializeError {
    pub(crate) const fn new(error: BoxedError, kind: SerializeErrorKind) -> Self {
        Self { error, kind }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// The type that failed to serialize.
///
/// Used in [`SerializeError`].
pub enum SerializeErrorKind {
    Channel,
    CurrentUser,
    Emoji,
    Guild,
    Integration,
    Member,
    Message,
    Presence,
    Role,
    ScheduledEvent,
    StageInstance,
    Sticker,
    User,
    VoiceState,
}

#[derive(Debug, ThisError)]
#[error("failed to update through {kind:?}")]
/// Failed to update some kind.
pub struct UpdateError {
    #[source]
    pub error: BoxedError,
    pub kind: UpdateErrorKind,
}

impl UpdateError {
    pub(crate) const fn new(error: BoxedError, kind: UpdateErrorKind) -> Self {
        Self { error, kind }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// The type that failed to update.
///
/// Used in [`UpdateError`].
pub enum UpdateErrorKind {
    ChannelPins,
    Guild,
    Member,
    Message,
    PartialMember,
    PartialUser,
    Reaction,
    ScheduledEventUserAdd,
    ScheduledEventUserRemove,
}

#[derive(Debug, ThisError)]
pub enum UpdateArchiveError {
    #[error("failed to deserialize")]
    Deserialization(#[source] BoxedError),
    #[error("failed to serialize")]
    Serialization(#[source] BoxedError),
}

#[derive(Debug, ThisError)]
#[error("failed to serialize {kind:?} meta")]
/// Failed to serialize a type's meta.
pub struct MetaError {
    #[source]
    pub error: BoxedError,
    pub kind: MetaErrorKind,
}

impl MetaError {
    pub(crate) const fn new(error: BoxedError, kind: MetaErrorKind) -> Self {
        Self { error, kind }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// The type whose meta failed to serialize.
///
/// Used in [`MetaError`].
pub enum MetaErrorKind {
    Channel,
    Emoji,
    Guild,
    Integration,
    Member,
    Message,
    Presence,
    Role,
    ScheduledEvent,
    StageInstance,
    Sticker,
    User,
    VoiceState,
}

#[derive(Debug, ThisError)]
/// Expire-related error.
pub enum ExpireError {
    #[error("failed to get a connection")]
    /// Failed to get a connection
    GetConnection(#[source] DedicatedConnectionError),
    #[error("failed to get meta")]
    /// Failed to get meta data.
    GetMeta(#[source] RedisError),
    #[error("failed to retrieve the 'notify-keyspace-events' config setting")]
    /// Failed to retrieve the `notify-keyspace-events` config setting.
    GetSetting(#[source] RedisError),
    #[error("failed to execute pipe")]
    /// Failed to execute pipe.
    Pipe(#[source] RedisError),
    #[error("failed to modify the 'notify-keyspace-events' config setting")]
    /// Failed to modify the `notify-keyspace-events` config setting.
    SetSetting(#[source] RedisError),
    #[error("failed to subscribe to expire events")]
    /// Failed to subscribe to events.
    Subscribe(#[source] RedisError),

    #[cfg(feature = "bytecheck")]
    #[cfg_attr(all(docsrs, not(doctest)), doc(cfg(feature = "bytecheck")))]
    #[error("cached bytes did not correspond to the meta type")]
    /// Cached bytes did not correspond to the expected meta type.
    Validation(#[from] ValidationError),
}

#[cfg(feature = "bytecheck")]
#[derive(Debug, thiserror::Error)]
#[error("cached bytes did not correspond to the cached type")]
/// Cached bytes did not correspond to the cached type.
pub struct ValidationError(#[from] BoxedError);

#[cfg(not(feature = "bytecheck"))]
pub type ValidationError = std::convert::Infallible;
