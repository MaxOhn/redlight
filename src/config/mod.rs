mod cacheable;
mod checked;
mod expirable;
mod from;
mod ignore;

pub use self::{
    cacheable::Cacheable,
    expirable::Expirable,
    from::{
        FromChannel, FromCurrentUser, FromEmoji, FromGuild, FromIntegration, FromMember,
        FromMessage, FromPresence, FromRole, FromStageInstance, FromSticker, FromUser,
        FromVoiceState,
    },
    ignore::Ignore,
};

/// Configuration for a [`RedisCache`](crate::RedisCache).
///
/// If an associated type should be cached, create a new type and implement the required traits for it.
///
/// If an associated type should not be cached, use [`Ignore`].
pub trait CacheConfig {
    type Channel<'a>: FromChannel<'a> + Cacheable;
    type CurrentUser<'a>: FromCurrentUser<'a> + Cacheable;
    type Emoji<'a>: FromEmoji<'a> + Cacheable;
    type Guild<'a>: FromGuild<'a> + Cacheable;
    type Integration<'a>: FromIntegration<'a> + Cacheable;
    type Member<'a>: FromMember<'a> + Cacheable;
    type Message<'a>: FromMessage<'a> + Cacheable + Expirable;
    type Presence<'a>: FromPresence<'a> + Cacheable;
    type Role<'a>: FromRole<'a> + Cacheable;
    type StageInstance<'a>: FromStageInstance<'a> + Cacheable;
    type Sticker<'a>: FromSticker<'a> + Cacheable;
    type User<'a>: FromUser<'a> + Cacheable;
    type VoiceState<'a>: FromVoiceState<'a> + Cacheable;
}
