use std::borrow::Cow;

use itoa::Buffer;
use twilight_model::id::{
    marker::{
        ChannelMarker, EmojiMarker, GuildMarker, IntegrationMarker, MessageMarker, RoleMarker,
        StageMarker, StickerMarker, UserMarker,
    },
    Id,
};

use crate::redis::{RedisWrite, ToRedisArgs};

pub(crate) enum RedisKey {
    Channel {
        id: Id<ChannelMarker>,
    },
    Channels,
    CurrentUser,
    Emoji {
        id: Id<EmojiMarker>,
    },
    Emojis,
    Guild {
        id: Id<GuildMarker>,
    },
    GuildChannels {
        id: Id<GuildMarker>,
    },
    GuildEmojis {
        id: Id<GuildMarker>,
    },
    GuildIntegrations {
        id: Id<GuildMarker>,
    },
    GuildMembers {
        id: Id<GuildMarker>,
    },
    GuildPresences {
        id: Id<GuildMarker>,
    },
    GuildRoles {
        id: Id<GuildMarker>,
    },
    GuildStageInstances {
        id: Id<GuildMarker>,
    },
    GuildStickers {
        id: Id<GuildMarker>,
    },
    GuildVoiceStates {
        id: Id<GuildMarker>,
    },
    Guilds,
    Integration {
        guild: Id<GuildMarker>,
        id: Id<IntegrationMarker>,
    },
    Member {
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    },
    Message {
        id: Id<MessageMarker>,
    },
    Presence {
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    },
    Role {
        id: Id<RoleMarker>,
    },
    Roles,
    StageInstance {
        id: Id<StageMarker>,
    },
    StageInstances,
    Sticker {
        id: Id<StickerMarker>,
    },
    Stickers,
    UnavailableGuilds,
    User {
        id: Id<UserMarker>,
    },
    UserGuilds {
        id: Id<UserMarker>,
    },
    Users,
    VoiceState {
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    },
}

impl From<Id<ChannelMarker>> for RedisKey {
    fn from(id: Id<ChannelMarker>) -> Self {
        Self::Channel { id }
    }
}

impl From<Id<EmojiMarker>> for RedisKey {
    fn from(id: Id<EmojiMarker>) -> Self {
        Self::Emoji { id }
    }
}

impl From<Id<GuildMarker>> for RedisKey {
    fn from(id: Id<GuildMarker>) -> Self {
        Self::Guild { id }
    }
}

impl From<Id<MessageMarker>> for RedisKey {
    fn from(id: Id<MessageMarker>) -> Self {
        Self::Message { id }
    }
}

impl From<Id<RoleMarker>> for RedisKey {
    fn from(id: Id<RoleMarker>) -> Self {
        Self::Role { id }
    }
}

impl From<Id<StageMarker>> for RedisKey {
    fn from(id: Id<StageMarker>) -> Self {
        Self::StageInstance { id }
    }
}

impl From<Id<StickerMarker>> for RedisKey {
    fn from(id: Id<StickerMarker>) -> Self {
        Self::Sticker { id }
    }
}

impl From<Id<UserMarker>> for RedisKey {
    fn from(id: Id<UserMarker>) -> Self {
        Self::User { id }
    }
}

impl ToRedisArgs for RedisKey {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        fn name_id<T>(name: &[u8], id: &Id<T>) -> Cow<'static, [u8]> {
            fn inner(name: &[u8], id: u64) -> Cow<'static, [u8]> {
                let mut buf = Buffer::new();
                let id = buf.format(id).as_bytes();

                let mut vec = Vec::with_capacity(name.len() + id.len());
                vec.extend_from_slice(name);
                vec.extend_from_slice(id);

                Cow::Owned(vec)
            }

            inner(name, id.get())
        }

        fn name_guild_id<T>(
            name: &[u8],
            guild: &Id<GuildMarker>,
            id: &Id<T>,
        ) -> Cow<'static, [u8]> {
            fn inner(name: &[u8], guild: &Id<GuildMarker>, id: u64) -> Cow<'static, [u8]> {
                let mut buf = Buffer::new();
                let guild = buf.format(guild.get()).as_bytes();

                let mut vec = Vec::with_capacity(name.len() + (guild.len() + 1) * 2);
                vec.extend_from_slice(name);
                vec.extend_from_slice(guild);

                let id = buf.format(id).as_bytes();
                vec.extend_from_slice(id);

                Cow::Owned(vec)
            }

            inner(name, guild, id.get())
        }

        let bytes = match self {
            Self::Channel { id } => name_id(b"CHANNEL:", id),
            Self::Channels => Cow::<[u8]>::Borrowed(b"CHANNELS"),
            Self::CurrentUser => Cow::<[u8]>::Borrowed(b"CURRENT_USER"),
            Self::Emoji { id } => name_id(b"EMOJI:", id),
            Self::Emojis => Cow::<[u8]>::Borrowed(b"EMOJIS"),
            Self::Guild { id } => name_id(b"GUILD:", id),
            Self::GuildChannels { id } => name_id(b"GUILD_CHANNELS:", id),
            Self::GuildEmojis { id } => name_id(b"GUILD_EMOJIS:", id),
            Self::GuildIntegrations { id } => name_id(b"GUILD_INTEGRATIONS:", id),
            Self::GuildMembers { id } => name_id(b"GUILD_MEMBERS:", id),
            Self::GuildPresences { id } => name_id(b"GUILD_PRESENCES:", id),
            Self::GuildRoles { id } => name_id(b"GUILD_ROLES:", id),
            Self::GuildStageInstances { id } => name_id(b"GUILD_STAGE_INSTANCES:", id),
            Self::GuildStickers { id } => name_id(b"GUILD_STICKERS:", id),
            Self::GuildVoiceStates { id } => name_id(b"GUILD_VOICE_STATES:", id),
            Self::Guilds => Cow::<[u8]>::Borrowed(b"GUILDS"),
            Self::Integration { guild, id } => name_guild_id(b"INTEGRATION:", guild, id),
            Self::Member { user, guild } => name_guild_id(b"MEMBER:", guild, user),
            Self::Message { id } => name_id(b"MESSAGE:", id),
            Self::Presence { guild, user } => name_guild_id(b"PRESENCE:", guild, user),
            Self::Role { id } => name_id(b"ROLE:", id),
            Self::Roles => Cow::<[u8]>::Borrowed(b"ROLES"),
            Self::StageInstance { id } => name_id(b"STAGE_INSTANCE:", id),
            Self::StageInstances => Cow::<[u8]>::Borrowed(b"STAGE_INSTANCES"),
            Self::Sticker { id } => name_id(b"STICKER:", id),
            Self::Stickers => Cow::<[u8]>::Borrowed(b"STICKERS"),
            Self::UnavailableGuilds => Cow::<[u8]>::Borrowed(b"UNAVAILABLE_GUILDS"),
            Self::User { id } => name_id(b"USER:", id),
            Self::UserGuilds { id } => name_id(b"USER_GUILDS:", id),
            Self::Users => Cow::<[u8]>::Borrowed(b"USERS"),
            Self::VoiceState { guild, user } => name_guild_id(b"VOICE_STATE:", guild, user),
        };

        out.write_arg(bytes.as_ref());
    }
}
