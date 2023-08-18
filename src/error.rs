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

    #[error("received invalid response from redis")]
    InvalidResponse,
    #[error("redis error")]
    Redis(#[from] crate::redis::RedisError),
    #[error(transparent)]
    Serialization(#[from] SerializeError),
    #[error("failed to update entry")]
    Update(#[from] UpdateError),
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

#[derive(Debug, ThisError)]
pub enum UpdateError {
    #[error("failed to update through ChannelPinsUpdate")]
    ChannelPins(#[source] Box<dyn StdError>),
    #[error("failed to update through GuildUpdate")]
    Guild(#[source] Box<dyn StdError>),
    #[error("failed to update through MemberUpdate")]
    Member(#[source] Box<dyn StdError>),
    #[error("failed to update through MessageUpdate")]
    Message(#[source] Box<dyn StdError>),
    #[error("failed to update through PartialMember")]
    PartialMember(#[source] Box<dyn StdError>),
    #[error("failed to update through PartialUser")]
    PartialUser(#[source] Box<dyn StdError>),
    #[error("failed to update through ReactionEvent")]
    Reaction(#[source] Box<dyn StdError>),
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
