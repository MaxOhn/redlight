mod cacheable;
mod checked;
mod from;
mod ignore;
mod reaction_event;

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
pub trait CacheConfig: FeatureBasedBounds {
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

#[cfg(not(feature = "metrics"))]
/// Trait that is implemented automatically for all acceptable types based on the selected
/// features.
///
/// This trait is required for [`CacheConfig`].
pub trait FeatureBasedBounds {}

#[cfg(not(feature = "metrics"))]
impl<T> FeatureBasedBounds for T {}

#[cfg(feature = "metrics")]
/// Trait that is implemented automatically for all acceptable types based on the selected
/// features.
///
/// This trait is required for [`CacheConfig`].
///
/// Metrics are gathered in a separate async task which depends on the config and as such the
/// config needs to be `Send + Sync + 'static`.
pub trait FeatureBasedBounds: Send + Sync + 'static {}

#[cfg(feature = "metrics")]
impl<T: Send + Sync + 'static> FeatureBasedBounds for T {}
