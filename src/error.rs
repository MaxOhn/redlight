use std::error::Error as StdError;

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
    CreatePool(#[source] RedisError),
    #[cfg(feature = "bb8")]
    #[error("failed to get a connection")]
    GetConnection(#[source] bb8_redis::bb8::RunError<RedisError>),

    #[cfg(feature = "deadpool")]
    #[error("failed to create redis pool")]
    CreatePool(#[from] deadpool_redis::CreatePoolError),
    #[cfg(feature = "deadpool")]
    #[error("failed to get a connection")]
    GetConnection(#[source] deadpool_redis::PoolError),

    #[cfg(feature = "validation")]
    #[error("cached bytes did not correspond to the cached type")]
    Validation(#[source] Box<dyn StdError>),

    #[cfg(feature = "cold_resume")]
    #[error("failed to serialize sessions")]
    SerializeSessions(
        #[source]
        rkyv::ser::serializers::CompositeSerializerError<
            std::convert::Infallible,
            rkyv::ser::serializers::FixedSizeScratchError,
            std::convert::Infallible,
        >,
    ),

    #[cfg(feature = "metrics")]
    #[error("failed to acquire a connection for metrics")]
    MetricsConnection(#[source] DedicatedConnectionError),

    #[error(transparent)]
    Expire(#[from] ExpireError),
    #[error("received invalid response from redis")]
    InvalidResponse,
    #[error(transparent)]
    Meta(#[from] MetaError),
    #[error("redis error")]
    Redis(#[from] RedisError),
    #[error(transparent)]
    Serialization(#[from] SerializeError),
    #[error("failed to update entry")]
    Update(#[from] UpdateError),
}

#[derive(Debug, ThisError)]
#[error("failed to serialize {kind:?}")]
pub struct SerializeError {
    #[source]
    pub error: Box<dyn StdError>,
    pub kind: SerializeErrorKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    StageInstance,
    Sticker,
    User,
    VoiceState,
}

#[derive(Debug, ThisError)]
#[error("failed to update through {kind:?}")]
pub struct UpdateError {
    #[source]
    pub error: Box<dyn StdError>,
    pub kind: UpdateErrorKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UpdateErrorKind {
    ChannelPins,
    Guild,
    Member,
    Message,
    PartialMember,
    PartialUser,
    Reaction,
}

#[derive(Debug, ThisError)]
pub enum UpdateArchiveError<D: StdError, S: StdError> {
    #[error("failed to deserialize")]
    Deserialization(#[source] D),
    #[error("failed to serialize")]
    Serialization(#[source] S),
}

impl<D, S> UpdateArchiveError<D, S>
where
    D: StdError + 'static,
    S: StdError + 'static,
{
    pub fn boxed(self) -> Box<dyn StdError> {
        Box::from(self)
    }
}

#[derive(Debug, ThisError)]
#[error("failed to serialize {kind:?} meta")]
pub struct MetaError {
    #[source]
    pub error: Box<dyn StdError>,
    pub kind: MetaErrorKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MetaErrorKind {
    Channel,
    Emoji,
    Guild,
    Integration,
    Member,
    Message,
    Presence,
    Role,
    StageInstance,
    Sticker,
    User,
    VoiceState,
}

#[derive(Debug, ThisError)]
pub enum ExpireError {
    #[error("failed to get a connection")]
    GetConnection(#[source] DedicatedConnectionError),
    #[error("failed to get meta")]
    GetMeta(#[source] RedisError),
    #[error("failed to retrieve the 'notify-keyspace-events' config setting")]
    GetSetting(#[source] RedisError),
    #[error("failed to execute pipe")]
    Pipe(#[source] RedisError),
    #[error("failed to modify the 'notify-keyspace-events' config setting")]
    SetSetting(#[source] RedisError),
    #[error("failed to subscribe to expire events")]
    Subscribe(#[source] RedisError),

    #[cfg(feature = "validation")]
    #[error("cached bytes did not correspond to the meta type")]
    Validation(#[source] Box<dyn StdError>),
}
