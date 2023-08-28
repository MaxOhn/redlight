use std::error::Error as StdError;

use twilight_model::{
    channel::{message::Sticker, Channel, Message, StageInstance},
    gateway::{
        payload::incoming::{
            invite_create::PartialUser, ChannelPinsUpdate, GuildUpdate, MemberUpdate, MessageUpdate,
        },
        presence::Presence,
    },
    guild::{Emoji, Guild, GuildIntegration, Member, PartialMember, Role},
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
    user::{CurrentUser, User},
    voice::VoiceState,
};

use crate::CachedArchive;

use super::{Cacheable, ReactionEvent};

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
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - the [`ChannelPinsUpdate`] event
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_pins_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), Box<dyn StdError>>>;
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
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - the [`GuildUpdate`] event
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_guild_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &GuildUpdate) -> Result<(), Box<dyn StdError>>>;
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
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - a reference to the [`PartialMember`] instance
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn update_via_partial(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), Box<dyn StdError>>>;

    /// Specify how [`MemberUpdate`] events are handled.
    ///
    /// If the event is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached member.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - the [`MemberUpdate`] event
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_member_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), Box<dyn StdError>>>;
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
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - the [`MessageUpdate`] event
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_message_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate) -> Result<(), Box<dyn StdError>>>;

    /// Specify how reaction events are handled.
    ///
    /// If the events are not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached message.
    ///
    /// The returned function should take two arguments:
    ///   - a mutable reference to the current entry which must be updated
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - a [`ReactionEvent`]
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn on_reaction_event(
    ) -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>) -> Result<(), Box<dyn StdError>>>;
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
    ///     either through [`CachedArchive::update_archive`] or [`CachedArchive::update_by_deserializing`].
    ///   - a reference to the [`PartialUser`] instance
    ///
    /// The return type must be [`Result`] where the error is a boxed [`std::error::Error`].
    // Abstracting the type through a type definition would likely cause
    // more confusion than do good so we'll allow the complexity.
    #[allow(clippy::type_complexity)]
    fn update_via_partial(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialUser) -> Result<(), Box<dyn StdError>>>;
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
