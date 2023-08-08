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
pub trait CacheConfig {
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
