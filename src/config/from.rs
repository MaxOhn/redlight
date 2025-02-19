use rkyv::rancor::Source;
use twilight_model::{
    channel::{message::Sticker, Channel, Message, StageInstance},
    gateway::{
        payload::incoming::{
            invite_create::PartialUser, ChannelPinsUpdate, GuildScheduledEventUserAdd,
            GuildScheduledEventUserRemove, GuildUpdate, MemberUpdate, MessageUpdate,
        },
        presence::Presence,
    },
    guild::{
        scheduled_event::GuildScheduledEvent, Emoji, Guild, GuildIntegration, Member,
        PartialMember, Role,
    },
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
    user::{CurrentUser, User},
    voice::VoiceState,
};

use super::{Cacheable, ReactionEvent};
use crate::CachedArchive;

/// Create a type from a [`Channel`] reference.
pub trait ICachedChannel<'a>: Cacheable {
    /// Create an instance from a [`Channel`] reference.
    fn from_channel(channel: &'a Channel) -> Self;

    /// Specify how [`ChannelPinsUpdate`] events are handled.
    ///
    /// If the event is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached channel.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - the [`ChannelPinsUpdate`] event
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_pins_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), E>>;
}

/// Create a type from a [`CurrentUser`] reference.
pub trait ICachedCurrentUser<'a>: Cacheable {
    /// Create an instance from a [`CurrentUser`] reference.
    fn from_current_user(current_user: &'a CurrentUser) -> Self;
}

/// Create a type from an [`Emoji`] reference.
pub trait ICachedEmoji<'a>: Cacheable {
    /// Create an instance from an [`Emoji`] reference.
    fn from_emoji(emoji: &'a Emoji) -> Self;
}

/// Create a type from a [`Guild`] reference.
pub trait ICachedGuild<'a>: Cacheable {
    /// Create an instance from a [`Guild`] reference.
    fn from_guild(guild: &'a Guild) -> Self;

    /// Specify how [`GuildUpdate`] events are handled.
    ///
    /// If the event is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached guild.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - the [`GuildUpdate`] event
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_guild_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &GuildUpdate) -> Result<(), E>>;
}

/// Create a type from a [`GuildIntegration`] reference.
pub trait ICachedIntegration<'a>: Cacheable {
    /// Create an instance from a [`GuildIntegration`] reference.
    fn from_integration(integration: &'a GuildIntegration) -> Self;
}

/// Create a type from a [`Member`] reference.
pub trait ICachedMember<'a>: Cacheable {
    /// Create an instance from a [`Member`] reference.
    fn from_member(guild_id: Id<GuildMarker>, member: &'a Member) -> Self;

    /// Specify how [`PartialMember`] types are handled.
    ///
    /// If the type is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached member.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - a reference to the [`PartialMember`] instance
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn update_via_partial<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), E>>;

    /// Specify how [`MemberUpdate`] events are handled.
    ///
    /// If the event is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached member.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - the [`MemberUpdate`] event
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_member_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), E>>;
}

/// Create a type from a [`Message`] reference.
pub trait ICachedMessage<'a>: Cacheable {
    /// Create an instance from a [`Message`] reference.
    fn from_message(message: &'a Message) -> Self;

    /// Specify how [`MessageUpdate`] events are handled.
    ///
    /// If the event is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached message.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - the [`MessageUpdate`] event
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_message_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate) -> Result<(), E>>;

    /// Specify how reaction events are handled.
    ///
    /// If the events are not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached message.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - a [`ReactionEvent`]
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_reaction_event<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>) -> Result<(), E>>;
}

/// Create a type from a [`Presence`] reference.
pub trait ICachedPresence<'a>: Cacheable {
    /// Create an instance from a [`Presence`] reference.
    fn from_presence(presence: &'a Presence) -> Self;
}

/// Create a type from a [`Role`] reference.
pub trait ICachedRole<'a>: Cacheable {
    /// Create an instance from a [`Role`] reference.
    fn from_role(role: &'a Role) -> Self;
}

/// Create a type from a [`GuildScheduledEvent`] reference.
pub trait ICachedScheduledEvent<'a>: Cacheable {
    /// Create an instance from a [`GuildScheduledEvent`] reference.
    fn from_scheduled_event(event: &'a GuildScheduledEvent) -> Self;

    /// Specify how user-add events are handled.
    ///
    /// If the events are not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached message.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - a [`GuildScheduledEventUserAdd`]
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_user_add_event<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &GuildScheduledEventUserAdd) -> Result<(), E>>;

    /// Specify how user-remove events are handled.
    ///
    /// If the events are not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached message.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - a [`GuildScheduledEventUserRemove`]
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_user_remove_event<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &GuildScheduledEventUserRemove) -> Result<(), E>>;
}

/// Create a type from a [`StageInstance`] reference.
pub trait ICachedStageInstance<'a>: Cacheable {
    /// Create an instance from a [`StageInstance`] reference.
    fn from_stage_instance(stage_instance: &'a StageInstance) -> Self;
}

/// Create a type from a [`Sticker`] reference.
pub trait ICachedSticker<'a>: Cacheable {
    /// Create an instance from a [`Sticker`] reference.
    fn from_sticker(sticker: &'a Sticker) -> Self;
}

/// Create a type from a [`User`] reference.
pub trait ICachedUser<'a>: Cacheable {
    /// Create an instance from a [`User`] reference.
    fn from_user(user: &'a User) -> Self;

    /// Specify how [`PartialUser`] types are handled.
    ///
    /// If the type is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached user.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or
    ///     [`CachedArchive::update_by_deserializing`].
    ///   - a reference to the [`PartialUser`] instance
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn update_via_partial<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialUser) -> Result<(), E>>;
}

/// Create a type from a [`VoiceState`] reference.
pub trait ICachedVoiceState<'a>: Cacheable {
    /// Create an instance from a [`VoiceState`] reference.
    fn from_voice_state(
        channel_id: Id<ChannelMarker>,
        guild_id: Id<GuildMarker>,
        voice_state: &'a VoiceState,
    ) -> Self;
}
