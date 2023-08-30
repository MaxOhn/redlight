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

/// Keys for storing and loading data from redis.
///
/// Implements `redis::ToRedisArgs` so it can be passed as argument
/// to `redis` commands.
///
/// Each variant is documented with the kind of data it points to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RedisKey {
    /// Serialized `CacheConfig::Channel`
    Channel { id: Id<ChannelMarker> },
    /// Sorted set of message ids ordered by timestamp i.e. most recent to oldest
    ChannelMessages { channel: Id<ChannelMarker> },
    /// Serialized `ChannelMeta`.
    ///
    /// Used for bookkeeping on expire events.
    ChannelMeta { id: Id<ChannelMarker> },
    /// Set of channel ids
    Channels,
    /// Serialized `CacheConfig::CurrentUser`
    CurrentUser,
    /// Serialized `CacheConfig::Emoji`
    Emoji { id: Id<EmojiMarker> },
    /// Serialized `EmojiMeta`.
    ///
    /// Used for bookkeeping on expire events.
    EmojiMeta { id: Id<EmojiMarker> },
    /// Set of emoji ids
    Emojis,
    /// Serialized `CacheConfig::Guild`
    Guild { id: Id<GuildMarker> },
    /// Set of channel ids
    GuildChannels { id: Id<GuildMarker> },
    /// Set of emoji ids
    GuildEmojis { id: Id<GuildMarker> },
    /// Set of integration ids
    GuildIntegrations { id: Id<GuildMarker> },
    /// Set of user ids
    GuildMembers { id: Id<GuildMarker> },
    /// Set of user ids
    GuildPresences { id: Id<GuildMarker> },
    /// Set of role ids
    GuildRoles { id: Id<GuildMarker> },
    /// Set of stage instance ids
    GuildStageInstances { id: Id<GuildMarker> },
    /// Set of sticker ids
    GuildStickers { id: Id<GuildMarker> },
    /// Set of user ids
    GuildVoiceStates { id: Id<GuildMarker> },
    /// Set of guild ids
    Guilds,
    /// Serialized `CacheConfig::Integration`
    Integration {
        guild: Id<GuildMarker>,
        id: Id<IntegrationMarker>,
    },
    /// Serialized `CacheConfig::Member`
    Member {
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    },
    /// Serialized `CacheConfig::Message`
    Message { id: Id<MessageMarker> },
    /// Serialized `MessageMeta`.
    ///
    /// Used for bookkeeping on expire events.
    MessageMeta { id: Id<MessageMarker> },
    /// Set of message ids
    Messages,
    /// Serialized `CacheConfig::Presence`
    Presence {
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    },
    /// Serialized `CacheConfig::Role`
    Role { id: Id<RoleMarker> },
    /// Serialized `RoleMeta`.
    ///
    /// Used for bookkeeping on expire events.
    RoleMeta { id: Id<RoleMarker> },
    /// Set of role ids
    Roles,
    #[cfg(feature = "cold_resume")]
    /// Serialized `SessionsWrapper`
    Sessions,
    /// Serialized `CacheConfig::StageInstance`
    StageInstance { id: Id<StageMarker> },
    /// Serialized `StageInstanceMeta`.
    ///
    /// Used for bookkeeping on expire events.
    StageInstanceMeta { id: Id<StageMarker> },
    /// Set of stage instance ids
    StageInstances,
    /// Serialized `CacheConfig::Sticker`
    Sticker { id: Id<StickerMarker> },
    /// Serialized `StickerMeta`.
    ///
    /// Used for bookkeeping on expire events.
    StickerMeta { id: Id<StickerMarker> },
    /// Set of sticker ids
    Stickers,
    /// Set of guild ids
    UnavailableGuilds,
    /// Serialized `CacheConfig::User`
    User { id: Id<UserMarker> },
    /// Set of guild ids
    UserGuilds { id: Id<UserMarker> },
    /// Set of user ids
    Users,
    /// Serialized `CacheConfig::VoiceState`
    VoiceState {
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    },
}

impl RedisKey {
    pub(crate) const CHANNEL_PREFIX: &[u8] = b"CHANNEL";
    pub(crate) const CHANNEL_MESSAGES_PREFIX: &[u8] = b"CHANNEL_MESSAGES_META";
    pub(crate) const CHANNEL_META_PREFIX: &[u8] = b"CHANNEL_META";
    pub(crate) const CHANNELS_PREFIX: &[u8] = b"CHANNELS";
    pub(crate) const CURRENT_USER_PREFIX: &[u8] = b"CURRENT_USER";
    pub(crate) const EMOJI_PREFIX: &[u8] = b"EMOJI";
    pub(crate) const EMOJI_META_PREFIX: &[u8] = b"EMOJI_META";
    pub(crate) const EMOJIS_PREFIX: &[u8] = b"EMOJIS";
    pub(crate) const GUILD_PREFIX: &[u8] = b"GUILD";
    pub(crate) const GUILD_CHANNELS_PREFIX: &[u8] = b"GUILD_CHANNELS";
    pub(crate) const GUILD_EMOJIS_PREFIX: &[u8] = b"GUILD_EMOJIS";
    pub(crate) const GUILD_INTEGRATIONS_PREFIX: &[u8] = b"GUILD_INTEGRATIONS";
    pub(crate) const GUILD_MEMBERS_PREFIX: &[u8] = b"GUILD_MEMBERS";
    pub(crate) const GUILD_PRESENCES_PREFIX: &[u8] = b"GUILD_PRESENCES";
    pub(crate) const GUILD_ROLES_PREFIX: &[u8] = b"GUILD_ROLES";
    pub(crate) const GUILD_STAGE_INSTANCES_PREFIX: &[u8] = b"GUILD_STAGE_INSTANCES";
    pub(crate) const GUILD_STICKERS_PREFIX: &[u8] = b"GUILD_STICKERS";
    pub(crate) const GUILD_VOICE_STATES_PREFIX: &[u8] = b"GUILD_VOICE_STATES";
    pub(crate) const GUILDS_PREFIX: &[u8] = b"GUILDS";
    pub(crate) const INTEGRATION_PREFIX: &[u8] = b"INTEGRATION";
    pub(crate) const MEMBER_PREFIX: &[u8] = b"MEMBER";
    pub(crate) const MESSAGE_PREFIX: &[u8] = b"MESSAGE";
    pub(crate) const MESSAGE_META_PREFIX: &[u8] = b"MESSAGE_META";
    pub(crate) const MESSAGES_PREFIX: &[u8] = b"MESSAGES";
    pub(crate) const PRESENCE_PREFIX: &[u8] = b"PRESENCE";
    pub(crate) const ROLE_PREFIX: &[u8] = b"ROLE";
    pub(crate) const ROLE_META_PREFIX: &[u8] = b"ROLE_META";
    pub(crate) const ROLES_PREFIX: &[u8] = b"ROLES";
    #[cfg(feature = "cold_resume")]
    pub(crate) const SESSIONS_PREFIX: &[u8] = b"SESSIONS";
    pub(crate) const STAGE_INSTANCE_PREFIX: &[u8] = b"STAGE_INSTANCE";
    pub(crate) const STAGE_INSTANCE_META_PREFIX: &[u8] = b"STAGE_INSTANCE_META";
    pub(crate) const STAGE_INSTANCES_PREFIX: &[u8] = b"STAGE_INSTANCES";
    pub(crate) const STICKER_PREFIX: &[u8] = b"STICKER";
    pub(crate) const STICKER_META_PREFIX: &[u8] = b"STICKER_META";
    pub(crate) const STICKERS_PREFIX: &[u8] = b"STICKERS";
    pub(crate) const UNAVAILABLE_GUILDS_PREFIX: &[u8] = b"UNAVAILABLE_GUILDS";
    pub(crate) const USER_PREFIX: &[u8] = b"USER";
    pub(crate) const USER_GUILDS_PREFIX: &[u8] = b"USER_GUILDS";
    pub(crate) const USERS_PREFIX: &[u8] = b"USERS";
    pub(crate) const VOICE_STATE_PREFIX: &[u8] = b"VOICE_STATE";
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
        fn name_id<T>(name: &[u8], id: Id<T>) -> Cow<'static, [u8]> {
            fn inner(name: &[u8], id: u64) -> Cow<'static, [u8]> {
                let mut buf = Buffer::new();
                let id = buf.format(id).as_bytes();

                let mut vec = Vec::with_capacity(name.len() + 1 + id.len());
                vec.extend_from_slice(name);
                vec.push(b':');
                vec.extend_from_slice(id);

                Cow::Owned(vec)
            }

            inner(name, id.get())
        }

        fn name_guild_id<T>(name: &[u8], guild: Id<GuildMarker>, id: Id<T>) -> Cow<'static, [u8]> {
            fn inner(name: &[u8], guild: Id<GuildMarker>, id: u64) -> Cow<'static, [u8]> {
                let mut buf = Buffer::new();
                let guild = buf.format(guild.get()).as_bytes();

                let mut vec = Vec::with_capacity(name.len() + 1 + (guild.len() + 1) * 2);
                vec.extend_from_slice(name);
                vec.push(b':');
                vec.extend_from_slice(guild);
                vec.push(b':');
                let id = buf.format(id).as_bytes();
                vec.extend_from_slice(id);

                Cow::Owned(vec)
            }

            inner(name, guild, id.get())
        }

        let bytes = match self {
            Self::Channel { id } => name_id(Self::CHANNEL_PREFIX, *id),
            Self::ChannelMessages { channel } => name_id(Self::CHANNEL_MESSAGES_PREFIX, *channel),
            Self::ChannelMeta { id } => name_id(Self::CHANNEL_META_PREFIX, *id),
            Self::Channels => Cow::Borrowed(Self::CHANNELS_PREFIX),
            Self::CurrentUser => Cow::Borrowed(Self::CURRENT_USER_PREFIX),
            Self::Emoji { id } => name_id(Self::EMOJI_PREFIX, *id),
            Self::EmojiMeta { id } => name_id(Self::EMOJI_META_PREFIX, *id),
            Self::Emojis => Cow::Borrowed(Self::EMOJIS_PREFIX),
            Self::Guild { id } => name_id(Self::GUILD_PREFIX, *id),
            Self::GuildChannels { id } => name_id(Self::GUILD_CHANNELS_PREFIX, *id),
            Self::GuildEmojis { id } => name_id(Self::GUILD_EMOJIS_PREFIX, *id),
            Self::GuildIntegrations { id } => name_id(Self::GUILD_INTEGRATIONS_PREFIX, *id),
            Self::GuildMembers { id } => name_id(Self::GUILD_MEMBERS_PREFIX, *id),
            Self::GuildPresences { id } => name_id(Self::GUILD_PRESENCES_PREFIX, *id),
            Self::GuildRoles { id } => name_id(Self::GUILD_ROLES_PREFIX, *id),
            Self::GuildStageInstances { id } => name_id(Self::GUILD_STAGE_INSTANCES_PREFIX, *id),
            Self::GuildStickers { id } => name_id(Self::GUILD_STICKERS_PREFIX, *id),
            Self::GuildVoiceStates { id } => name_id(Self::GUILD_VOICE_STATES_PREFIX, *id),
            Self::Guilds => Cow::Borrowed(Self::GUILDS_PREFIX),
            Self::Integration { guild, id } => name_guild_id(Self::INTEGRATION_PREFIX, *guild, *id),
            Self::Member { user, guild } => name_guild_id(Self::MEMBER_PREFIX, *guild, *user),
            Self::Message { id } => name_id(Self::MESSAGE_PREFIX, *id),
            Self::MessageMeta { id } => name_id(Self::MESSAGE_META_PREFIX, *id),
            Self::Messages => Cow::Borrowed(Self::MESSAGES_PREFIX),
            Self::Presence { guild, user } => name_guild_id(Self::PRESENCE_PREFIX, *guild, *user),
            Self::Role { id } => name_id(Self::ROLE_PREFIX, *id),
            Self::RoleMeta { id } => name_id(Self::ROLE_META_PREFIX, *id),
            Self::Roles => Cow::Borrowed(Self::ROLES_PREFIX),
            #[cfg(feature = "cold_resume")]
            Self::Sessions => Cow::Borrowed(Self::SESSIONS_PREFIX),
            Self::StageInstance { id } => name_id(Self::STAGE_INSTANCE_PREFIX, *id),
            Self::StageInstanceMeta { id } => name_id(Self::STAGE_INSTANCE_META_PREFIX, *id),
            Self::StageInstances => Cow::Borrowed(Self::STAGE_INSTANCES_PREFIX),
            Self::Sticker { id } => name_id(Self::STICKER_PREFIX, *id),
            Self::StickerMeta { id } => name_id(Self::STICKER_META_PREFIX, *id),
            Self::Stickers => Cow::Borrowed(Self::STICKERS_PREFIX),
            Self::UnavailableGuilds => Cow::Borrowed(Self::UNAVAILABLE_GUILDS_PREFIX),
            Self::User { id } => name_id(Self::USER_PREFIX, *id),
            Self::UserGuilds { id } => name_id(Self::USER_GUILDS_PREFIX, *id),
            Self::Users => Cow::Borrowed(Self::USERS_PREFIX),
            Self::VoiceState { guild, user } => {
                name_guild_id(Self::VOICE_STATE_PREFIX, *guild, *user)
            }
        };

        out.write_arg(bytes.as_ref());
    }
}
