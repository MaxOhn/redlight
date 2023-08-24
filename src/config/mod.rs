mod cacheable;
mod from;
mod ignore;
mod reaction_event;

pub(crate) mod checked;

pub use self::{
    cacheable::Cacheable,
    from::{
        ICachedChannel, ICachedCurrentUser, ICachedEmoji, ICachedGuild, ICachedIntegration,
        ICachedMember, ICachedMessage, ICachedPresence, ICachedRole, ICachedStageInstance,
        ICachedSticker, ICachedUser, ICachedVoiceState,
    },
    ignore::Ignore,
    reaction_event::ReactionEvent,
};

/// Configuration for a [`RedisCache`](crate::RedisCache).
///
/// If an associated type should be cached, create a new type and implement the required traits for it.
///
/// If an associated type should not be cached, use [`Ignore`].
pub trait CacheConfig: Send + Sync + 'static {
    #[cfg(feature = "metrics")]
    /// The interval duration until metrics are updated.
    ///
    /// The suggested duration is 30 seconds.
    const METRICS_INTERVAL_DURATION: std::time::Duration;

    type Channel<'a>: ICachedChannel<'a>;
    type CurrentUser<'a>: ICachedCurrentUser<'a>;
    type Emoji<'a>: ICachedEmoji<'a>;
    type Guild<'a>: ICachedGuild<'a>;
    type Integration<'a>: ICachedIntegration<'a>;
    type Member<'a>: ICachedMember<'a>;
    type Message<'a>: ICachedMessage<'a> + Cacheable;
    type Presence<'a>: ICachedPresence<'a>;
    type Role<'a>: ICachedRole<'a>;
    type StageInstance<'a>: ICachedStageInstance<'a>;
    type Sticker<'a>: ICachedSticker<'a>;
    type User<'a>: ICachedUser<'a>;
    type VoiceState<'a>: ICachedVoiceState<'a>;
}
