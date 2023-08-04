use std::pin::Pin;

use twilight_model::{
    channel::{message::Sticker, Channel, Message, StageInstance},
    gateway::{
        payload::incoming::{
            invite_create::PartialUser, ChannelPinsUpdate, MemberUpdate, MessageUpdate,
        },
        presence::Presence,
    },
    guild::{Emoji, Guild, GuildIntegration, Member, PartialGuild, PartialMember, Role},
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
    user::{CurrentUser, User},
    voice::VoiceState,
};

use super::Cacheable;

/// Create a type from a [`Channel`] reference.
pub trait ICachedChannel<'a>: Cacheable {
    /// Create an instance from a [`Channel`] reference.
    fn from_channel(channel: &'a Channel) -> Self;

    /// Specify how [`ChannelPinsUpdate`] events are handled.
    ///
    /// If the event is not of interest, return `None`.
    /// Otherwise, return a function that updates the currently cached channel.
    // Allowing the type complexity since abstracting it through a type definition
    // would likely cause more confusion than do good.
    #[allow(clippy::type_complexity)]
    fn on_pins_update() -> Option<fn(Pin<&mut Self::Archived>, &ChannelPinsUpdate)>;
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

/// Create a type from a [`Guild`] or [`PartialGuild`] reference.
pub trait ICachedGuild<'a>: Cacheable {
    /// Create an instance from a [`Guild`] reference.
    fn from_guild(guild: &'a Guild) -> Self;

    /// Try to create an instance from a [`PartialGuild`] reference.
    /// If there is not enough data available, return `None`.
    fn from_partial_guild(guild: &'a PartialGuild) -> Option<Self>;
}

/// Create a type from a [`GuildIntegration`] reference.
pub trait ICachedIntegration<'a>: Cacheable {
    /// Create an instance from a [`GuildIntegration`] reference.
    fn from_integration(integration: &'a GuildIntegration) -> Self;
}

/// Create a type from a [`Member`], [`PartialMember`], or [`MemberUpdate`] reference.
pub trait ICachedMember<'a>: Cacheable {
    /// Create an instance from a [`Member`] reference.
    fn from_member(guild_id: Id<GuildMarker>, member: &'a Member) -> Self;

    /// Try to create an instance from a [`PartialMember`] reference.
    /// If there is not enough data available, return `None`.
    fn from_partial_member(guild_id: Id<GuildMarker>, member: &'a PartialMember) -> Option<Self>;

    /// Try to create an instance from a [`MemberUpdate`] reference.
    /// If there is not enough data available, return `None`.
    fn from_member_update(update: &'a MemberUpdate) -> Option<Self>;
}

/// Create a type from a [`Message`] or [`MessageUpdate`] reference.
pub trait ICachedMessage<'a>: Cacheable {
    /// Create an instance from a [`Message`] reference.
    fn from_message(message: &'a Message) -> Self;

    /// Try to create an instance from a [`MessageUpdate`] reference.
    /// If there is not enough data available, return `None`.
    fn from_message_update(update: &'a MessageUpdate) -> Option<Self>;
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

/// Create a type from a [`User`] or [`PartialUser`] reference.
pub trait ICachedUser<'a>: Cacheable {
    /// Create an instance from a [`User`] reference.
    fn from_user(user: &'a User) -> Self;

    /// Try to create an instance from a [`PartialUser`] reference.
    /// If there is not enough data available, return `None`.
    fn from_partial_user(user: &'a PartialUser) -> Option<Self>;
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
