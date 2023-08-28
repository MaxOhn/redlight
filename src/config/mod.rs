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
///
/// # Example
///
/// ```
/// # use std::{time::Duration, error::Error};
/// # use rkyv::{Archive, Serialize, ser::serializers::AllocSerializer};
/// # use twilight_model::{channel::{message::Message, Channel}, gateway::payload::incoming::{ChannelPinsUpdate, MessageUpdate}};
/// # use twilight_redis::{CachedArchive, config::ReactionEvent};
/// use rkyv::with::{Map, RefAsBox};
/// use twilight_model::{channel::ChannelFlags, id::{Id, marker::ChannelMarker}};
/// use twilight_redis::config::{CacheConfig, Cacheable, ICachedChannel, ICachedMessage, Ignore};
/// use twilight_redis::rkyv_util::{id::IdRkyv, util::BitflagsRkyv};
///
/// struct Config;
///
/// impl CacheConfig for Config {
/// #    #[cfg(feature = "metrics")]
///     // Only if the `metrics` feature is enabled
///     const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(30);
///
///     type Channel<'a> = CachedChannel; // <-
///     type CurrentUser<'a> = Ignore;
///     type Emoji<'a> = Ignore;
///     type Guild<'a> = Ignore;
///     type Integration<'a> = Ignore;
///     type Member<'a> = Ignore;
///     type Message<'a> = CachedMessage<'a>; // <-
///     type Presence<'a> = Ignore;
///     type Role<'a> = Ignore;
///     type StageInstance<'a> = Ignore;
///     type Sticker<'a> = Ignore;
///     type User<'a> = Ignore;
///     type VoiceState<'a> = Ignore;
/// }
///
/// #[derive(Archive, Serialize)]
/// # #[cfg_attr(feature = "validation", archive(check_bytes))]
/// # /*
/// #[archive(check_bytes)] // only if `validation` feature is enabled
/// # */
/// struct CachedChannel {
///     #[with(Map<BitflagsRkyv>)]
///     flags: Option<ChannelFlags>,
///     #[with(IdRkyv)]
///     id: Id<ChannelMarker>,
/// }
///
/// impl<'a> ICachedChannel<'a> for CachedChannel {
///     # /*
///     // ...
///     # */
///     # fn from_channel(_: &'a Channel) -> Self { unimplemented!() }
///     # fn on_pins_update() -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), Box<dyn Error>>> { unimplemented!() }
/// }
///
/// impl Cacheable for CachedChannel {
///     # /*
///     // ...
///     # */
///     # type Serializer = AllocSerializer<0>;
///     # fn expire() -> Option<Duration> { None }
/// }
///
/// #[derive(Archive, Serialize)]
/// # #[cfg_attr(feature = "validation", archive(check_bytes))]
/// # /*
/// #[archive(check_bytes)] // only if `validation` feature is enabled
/// # */
/// struct CachedMessage<'a> {
///     #[with(RefAsBox)]
///     content: &'a str,
/// }
///
/// impl<'a> ICachedMessage<'a> for CachedMessage<'a> {
///     # /*
///     // ...
///     # */
///     # fn from_message(_: &'a Message) -> Self { unimplemented!() }
///     # fn on_message_update() -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate) -> Result<(), Box<dyn Error>>> { unimplemented!() }
///     # fn on_reaction_event() -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>) -> Result<(), Box<dyn Error>>> { unimplemented!() }
/// }
///
/// impl Cacheable for CachedMessage<'_> {
///     # /*
///     // ...
///     # */
///     # type Serializer = AllocSerializer<0>;
///     # fn expire() -> Option<Duration> { None }
/// }
/// ```
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
    type Message<'a>: ICachedMessage<'a>;
    type Presence<'a>: ICachedPresence<'a>;
    type Role<'a>: ICachedRole<'a>;
    type StageInstance<'a>: ICachedStageInstance<'a>;
    type Sticker<'a>: ICachedSticker<'a>;
    type User<'a>: ICachedUser<'a>;
    type VoiceState<'a>: ICachedVoiceState<'a>;
}
