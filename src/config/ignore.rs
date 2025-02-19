use std::time::Duration;

use rkyv::{rancor::Source, Archive, Place};
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

use super::ReactionEvent;
use crate::{
    config::{
        Cacheable, ICachedChannel, ICachedCurrentUser, ICachedEmoji, ICachedGuild,
        ICachedIntegration, ICachedMember, ICachedMessage, ICachedPresence, ICachedRole,
        ICachedStageInstance, ICachedSticker, ICachedUser, ICachedVoiceState,
    },
    CachedArchive,
};

/// Struct to indicate that a type should not be cached.
///
/// Used by specifying [`Ignore`] for associated types of
/// [`CacheConfig`](crate::config::CacheConfig).
pub struct Ignore;

impl ICachedChannel<'_> for Ignore {
    fn from_channel(_: &'_ Channel) -> Self {
        Self
    }

    fn on_pins_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), E>> {
        None
    }
}

impl ICachedCurrentUser<'_> for Ignore {
    fn from_current_user(_: &'_ CurrentUser) -> Self {
        Self
    }
}

impl ICachedEmoji<'_> for Ignore {
    fn from_emoji(_: &'_ Emoji) -> Self {
        Self
    }
}

impl ICachedIntegration<'_> for Ignore {
    fn from_integration(_: &'_ GuildIntegration) -> Self {
        Self
    }
}

impl ICachedGuild<'_> for Ignore {
    fn from_guild(_: &'_ Guild) -> Self {
        Self
    }

    fn on_guild_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &GuildUpdate) -> Result<(), E>> {
        None
    }
}

impl ICachedMember<'_> for Ignore {
    fn from_member(_: Id<GuildMarker>, _: &'_ Member) -> Self {
        Self
    }

    fn update_via_partial<E>(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), E>> {
        None
    }

    fn on_member_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), E>> {
        None
    }
}

impl ICachedMessage<'_> for Ignore {
    fn from_message(_: &'_ Message) -> Self {
        Self
    }

    fn on_message_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate) -> Result<(), E>> {
        None
    }

    fn on_reaction_event<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>) -> Result<(), E>> {
        None
    }
}

impl ICachedPresence<'_> for Ignore {
    fn from_presence(_: &'_ Presence) -> Self {
        Self
    }
}

impl ICachedRole<'_> for Ignore {
    fn from_role(_: &'_ Role) -> Self {
        Self
    }
}

impl ICachedStageInstance<'_> for Ignore {
    fn from_stage_instance(_: &StageInstance) -> Self {
        Self
    }
}

impl ICachedSticker<'_> for Ignore {
    fn from_sticker(_: &'_ Sticker) -> Self {
        Self
    }
}

impl ICachedUser<'_> for Ignore {
    fn from_user(_: &User) -> Self {
        Self
    }

    fn update_via_partial<E>() -> Option<fn(&mut CachedArchive<Self>, &PartialUser) -> Result<(), E>>
    {
        None
    }
}

impl ICachedVoiceState<'_> for Ignore {
    fn from_voice_state(_: Id<ChannelMarker>, _: Id<GuildMarker>, _: &'_ VoiceState) -> Self {
        Self
    }
}

impl Cacheable for Ignore {
    type Bytes = [u8; 0];

    const WANTED: bool = false;

    fn expire() -> Option<Duration> {
        None
    }

    fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
        Ok([])
    }
}

impl Archive for Ignore {
    type Archived = ();
    type Resolver = ();

    fn resolve(&self, (): Self::Resolver, _: Place<Self::Archived>) {}
}
