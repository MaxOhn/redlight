mod cacheable;
mod checked;
mod from;
mod reaction_event;

// pub but hidden for `cargo rdme`
#[doc(hidden)]
pub mod ignore;

pub use self::{
    cacheable::{Cacheable, SerializeMany},
    checked::CheckedArchive,
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
/// If an associated type should be cached, create a new type and implement the
/// required traits for it.
///
/// If an associated type should not be cached, use [`Ignore`].
///
/// # Example
///
/// ```
/// # use std::{time::Duration};
/// # use redlight::{CachedArchive, config::ReactionEvent};
/// # use rkyv::{Archive, Serialize};
/// # use twilight_model::{
/// #     channel::{message::Message, Channel},
/// #     gateway::payload::incoming::{ChannelPinsUpdate, MessageUpdate}
/// # };
/// use redlight::config::{CacheConfig, Cacheable, ICachedChannel, ICachedMessage, Ignore};
/// use redlight::rkyv_util::{id::IdRkyv, util::BitflagsRkyv};
/// use rkyv::with::{Map, InlineAsBox};
/// use rkyv::rancor::Fallible;
/// use twilight_model::{channel::ChannelFlags, id::{Id, marker::ChannelMarker}};
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
/// struct CachedChannel {
///     #[rkyv(with = Map<BitflagsRkyv>)]
///     flags: Option<ChannelFlags>,
///     #[rkyv(with = IdRkyv)]
///     id: Id<ChannelMarker>,
/// }
///
/// impl<'a> ICachedChannel<'a> for CachedChannel {
///     # /*
///     // ...
///     # */
///     # fn from_channel(_: &'a Channel) -> Self { unimplemented!() }
///     # fn on_pins_update() -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate)
///     #     -> Result<(), Self::Error>> { None }
/// }
///
/// impl Cacheable for CachedChannel {
///     # /*
///     // ...
///     # */
///     # type Bytes = [u8; 0];
///     # fn expire() -> Option<Duration> { None }
///     # fn serialize_one(&self) -> Result<Self::Bytes, Self::Error> { Ok([]) }
/// }
///
/// impl Fallible for CachedChannel {
///     type Error = rkyv::rancor::Error;
/// }
///
/// #[derive(Archive, Serialize)]
/// struct CachedMessage<'a> {
///     #[rkyv(with = InlineAsBox)]
///     content: &'a str,
/// }
///
/// impl<'a> ICachedMessage<'a> for CachedMessage<'a> {
///     # /*
///     // ...
///     # */
///     # fn from_message(_: &'a Message) -> Self { unimplemented!() }
///     # fn on_message_update() -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate)
///     #     -> Result<(), Self::Error>> { None }
///     # fn on_reaction_event() -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>)
///     #     -> Result<(), Self::Error>> { None }
/// }
///
/// impl Cacheable for CachedMessage<'_> {
///     # /*
///     // ...
///     # */
///     # type Bytes = [u8; 0];
///     # fn expire() -> Option<Duration> { None }
///     # fn serialize_one(&self) -> Result<Self::Bytes, Self::Error> { Ok([]) }
/// }
///
/// impl Fallible for CachedMessage<'_> {
///     type Error = rkyv::rancor::Error;
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
