use std::{convert::Infallible, time::Duration};

use rkyv::{Archive, Deserialize, Fallible, Serialize};
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

use crate::{
    config::{
        Cacheable, ICachedChannel, ICachedCurrentUser, ICachedEmoji, ICachedGuild,
        ICachedIntegration, ICachedMember, ICachedMessage, ICachedPresence, ICachedRole,
        ICachedStageInstance, ICachedSticker, ICachedUser, ICachedVoiceState,
    },
    error::BoxedError,
    ser::NoopSerializer,
    CachedArchive,
};

use super::ReactionEvent;

/// Struct to indicate that a type should not be cached.
///
/// Used by specifying [`Ignore`] for associated types of [`CacheConfig`](crate::config::CacheConfig).
pub struct Ignore;

impl ICachedChannel<'_> for Ignore {
    fn from_channel(_: &'_ Channel) -> Self {
        Self
    }

    fn on_pins_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), BoxedError>> {
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

    fn on_guild_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &GuildUpdate) -> Result<(), BoxedError>> {
        None
    }
}

impl ICachedMember<'_> for Ignore {
    fn from_member(_: Id<GuildMarker>, _: &'_ Member) -> Self {
        Self
    }

    fn update_via_partial(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), BoxedError>> {
        None
    }

    fn on_member_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), BoxedError>> {
        None
    }
}

impl ICachedMessage<'_> for Ignore {
    fn from_message(_: &'_ Message) -> Self {
        Self
    }

    fn on_message_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate) -> Result<(), BoxedError>> {
        None
    }

    fn on_reaction_event(
    ) -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>) -> Result<(), BoxedError>> {
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

    fn update_via_partial(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialUser) -> Result<(), BoxedError>> {
        None
    }
}

impl ICachedVoiceState<'_> for Ignore {
    fn from_voice_state(_: Id<ChannelMarker>, _: Id<GuildMarker>, _: &'_ VoiceState) -> Self {
        Self
    }
}

impl Cacheable for Ignore {
    type Serializer = NoopSerializer;

    const WANTED: bool = false;

    fn expire() -> Option<Duration> {
        None
    }
}

impl Archive for Ignore {
    type Archived = ();
    type Resolver = ();

    unsafe fn resolve(&self, _: usize, _: Self::Resolver, _: *mut Self::Archived) {}
}

impl Serialize<NoopSerializer> for Ignore {
    fn serialize(&self, _: &mut NoopSerializer) -> Result<(), Infallible> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Self, D> for Ignore {
    fn deserialize(&self, _: &mut D) -> Result<Self, D::Error> {
        Ok(Self)
    }
}

impl<'a> From<&'a ()> for Ignore {
    fn from(_: &'a ()) -> Self {
        Self
    }
}

#[cfg(feature = "validation")]
#[cfg_attr(docsrs, doc(cfg(feature = "validation")))]
impl<C> rkyv::CheckBytes<C> for Ignore {
    type Error = Infallible;

    unsafe fn check_bytes<'a>(value: *const Self, _: &mut C) -> Result<&'a Self, Self::Error> {
        Ok(&*value)
    }
}
