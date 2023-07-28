use std::{marker::PhantomData, ops::DerefMut};

use twilight_model::{
    application::interaction::InteractionData,
    channel::{message::Sticker, Channel, Message, StageInstance},
    gateway::{
        event::Event,
        payload::incoming::{invite_create::PartialUser, MemberUpdate, MessageUpdate},
        presence::{Presence, UserOrId},
    },
    guild::{
        Emoji, Guild, GuildIntegration, Member, PartialGuild, PartialMember, Role, UnavailableGuild,
    },
    id::{
        marker::{
            ChannelMarker, EmojiMarker, GuildMarker, IntegrationMarker, MessageMarker, RoleMarker,
            StageMarker, StickerMarker, UserMarker,
        },
        Id,
    },
    user::{CurrentUser, User},
    voice::VoiceState,
};

use crate::{
    config::{
        CacheConfig, Cacheable, Expirable, FromChannel, FromCurrentUser, FromEmoji, FromGuild,
        FromIntegration, FromMember, FromMessage, FromPresence, FromRole, FromStageInstance,
        FromSticker, FromUser, FromVoiceState,
    },
    key::RedisKey,
    redis::{aio::Connection, AsyncCommands},
    util::aligned_vec::BytesRedisArgs,
    CacheError, CacheResult, CachedValue,
};

#[cfg(feature = "bb8")]
type Pool = bb8_redis::bb8::Pool<bb8_redis::RedisConnectionManager>;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
type Pool = deadpool_redis::Pool;

/// Redis-based cache for data of twilight's gateway [`Event`]s.
pub struct RedisCache<C> {
    pool: Pool,
    config: PhantomData<C>,
}

impl<C> RedisCache<C> {
    #[cfg(feature = "bb8")]
    pub async fn new(url: &str) -> CacheResult<Self> {
        use bb8_redis::{bb8::Pool, RedisConnectionManager};

        let manager = RedisConnectionManager::new(url).map_err(CacheError::CreatePool)?;

        let pool = Pool::builder()
            .build(manager)
            .await
            .map_err(CacheError::CreatePool)?;

        Ok(Self {
            pool,
            config: PhantomData,
        })
    }

    #[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
    pub async fn new(url: &str) -> CacheResult<Self> {
        use deadpool_redis::{Config, Runtime};

        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        Ok(Self {
            pool,
            config: PhantomData,
        })
    }

    #[cfg(any(feature = "bb8", feature = "deadpool"))]
    pub fn with_pool(pool: Pool) -> Self {
        Self {
            pool,
            config: PhantomData,
        }
    }

    async fn connection(&self) -> CacheResult<impl DerefMut<Target = Connection> + '_> {
        self.pool.get().await.map_err(CacheError::GetConnection)
    }
}

type ChannelSerializer<'a, C> = <<C as CacheConfig>::Channel<'a> as Cacheable>::Serializer;
type EmojiSerializer<'a, C> = <<C as CacheConfig>::Emoji<'a> as Cacheable>::Serializer;
type MemberSerializer<'a, C> = <<C as CacheConfig>::Member<'a> as Cacheable>::Serializer;
type PresenceSerializer<'a, C> = <<C as CacheConfig>::Presence<'a> as Cacheable>::Serializer;
type RoleSerializer<'a, C> = <<C as CacheConfig>::Role<'a> as Cacheable>::Serializer;
type StageInstanceSerializer<'a, C> =
    <<C as CacheConfig>::StageInstance<'a> as Cacheable>::Serializer;
type StickerSerializer<'a, C> = <<C as CacheConfig>::Sticker<'a> as Cacheable>::Serializer;
type UserSerializer<'a, C> = <<C as CacheConfig>::User<'a> as Cacheable>::Serializer;
type VoiceStateSerializer<'a, C> = <<C as CacheConfig>::VoiceState<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    pub async fn update<'a>(&self, event: &'a Event) -> CacheResult<()> {
        match event {
            Event::AutoModerationActionExecution(_) => {}
            Event::AutoModerationRuleCreate(_) => {}
            Event::AutoModerationRuleDelete(_) => {}
            Event::AutoModerationRuleUpdate(_) => {}
            Event::BanAdd(event) => self.store_user(&event.user).await?,
            Event::BanRemove(event) => self.store_user(&event.user).await?,
            Event::ChannelCreate(event) => self.store_channel(event).await?,
            Event::ChannelDelete(event) => self.delete_channel(event.guild_id, event.id).await?,
            Event::ChannelPinsUpdate(_) => {}
            Event::ChannelUpdate(event) => self.store_channel(event).await?,
            Event::CommandPermissionsUpdate(_) => {}
            Event::GatewayClose(_) => {}
            Event::GatewayHeartbeat(_) => {}
            Event::GatewayHeartbeatAck => {}
            Event::GatewayHello(_) => {}
            Event::GatewayInvalidateSession(_) => {}
            Event::GatewayReconnect => {}
            Event::GiftCodeUpdate => {}
            Event::GuildAuditLogEntryCreate(_) => {}
            Event::GuildCreate(event) => self.store_guild(event).await?,
            Event::GuildDelete(event) => {
                if event.unavailable {
                    self.store_unavailable_guild(event.id).await?
                } else {
                    self.delete_guild(event.id).await?
                }
            }
            Event::GuildEmojisUpdate(event) => {
                self.store_emojis(event.guild_id, &event.emojis).await?
            }
            Event::GuildIntegrationsUpdate(_) => {}
            Event::GuildScheduledEventCreate(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(user).await?;
                }
            }
            Event::GuildScheduledEventDelete(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(user).await?;
                }
            }
            Event::GuildScheduledEventUpdate(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(user).await?;
                }
            }
            Event::GuildScheduledEventUserAdd(_) => {}
            Event::GuildScheduledEventUserRemove(_) => {}
            Event::GuildStickersUpdate(event) => {
                self.store_stickers(event.guild_id, &event.stickers).await?
            }
            Event::GuildUpdate(event) => self.store_partial_guild(event).await?,
            Event::IntegrationCreate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(guild_id, event).await?;
                }
            }
            Event::IntegrationDelete(event) => {
                self.delete_integration(event.guild_id, event.id).await?
            }
            Event::IntegrationUpdate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(guild_id, event).await?;
                }
            }
            Event::InteractionCreate(event) => {
                if let Some(ref channel) = event.channel {
                    self.store_channel(channel).await?;
                }

                if let Some(InteractionData::ApplicationCommand(ref data)) = event.data {
                    if let Some(ref resolved) = data.resolved {
                        if let Some(guild_id) = event.guild_id {
                            let roles = resolved.roles.values();
                            self.store_roles(guild_id, roles).await?;
                        }

                        let users = resolved.users.values();
                        self.store_users(users).await?;
                    }
                }

                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_partial_member(guild_id, member).await?;
                }

                if let Some(ref msg) = event.message {
                    self.store_message(msg).await?;
                }

                if let Some(ref user) = event.user {
                    self.store_user(user).await?;
                }
            }
            Event::InviteCreate(event) => {
                if let Some(ref user) = event.inviter {
                    self.store_user(user).await?;
                }

                if let Some(ref user) = event.target_user {
                    self.store_partial_user(user).await?;
                }
            }
            Event::InviteDelete(_) => {}
            Event::MemberAdd(event) => self.store_member(event.guild_id, &event.member).await?,
            Event::MemberRemove(event) => self.delete_member(event.guild_id, event.user.id).await?,
            Event::MemberUpdate(event) => self.store_member_update(event).await?,
            Event::MemberChunk(event) => {
                self.store_members(event.guild_id, &event.members).await?;
                self.store_presences(event.guild_id, &event.presences)
                    .await?;
            }
            Event::MessageCreate(event) => self.store_message(event).await?,
            Event::MessageDelete(event) => self.delete_message(event.id).await?,
            Event::MessageDeleteBulk(event) => self.delete_messages(&event.ids).await?,
            Event::MessageUpdate(event) => self.store_message_update(event).await?,
            Event::PresenceUpdate(event) => self.store_presence(event).await?,
            Event::PresencesReplace => {}
            Event::ReactionAdd(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(guild_id, member).await?;
                }
            }
            Event::ReactionRemove(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(guild_id, member).await?;
                }
            }
            Event::ReactionRemoveAll(_) => {}
            Event::ReactionRemoveEmoji(_) => {}
            Event::Ready(event) => {
                self.store_current_user(&event.user).await?;
                self.store_unavailable_guilds(&event.guilds).await?;
            }
            Event::Resumed => {}
            Event::RoleCreate(event) => self.store_role(event.guild_id, &event.role).await?,
            Event::RoleDelete(event) => self.delete_role(event.guild_id, event.role_id).await?,
            Event::RoleUpdate(event) => self.store_role(event.guild_id, &event.role).await?,
            Event::StageInstanceCreate(event) => self.store_stage_instance(event).await?,
            Event::StageInstanceDelete(event) => {
                self.delete_stage_instance(event.guild_id, event.id).await?
            }
            Event::StageInstanceUpdate(event) => self.store_stage_instance(event).await?,
            Event::ThreadCreate(event) => self.store_channel(event).await?,
            Event::ThreadDelete(event) => {
                self.delete_channel(Some(event.guild_id), event.id).await?
            }
            Event::ThreadListSync(event) => {
                self.store_channels(event.guild_id, &event.threads).await?
            }
            Event::ThreadMemberUpdate(event) => {
                if let Some(ref presence) = event.presence {
                    self.store_presence(presence).await?;

                    if let Some(ref member) = event.member {
                        self.store_member(presence.guild_id, member).await?;
                    }
                }
            }
            Event::ThreadMembersUpdate(_) => {}
            Event::ThreadUpdate(event) => self.store_channel(event).await?,
            Event::TypingStart(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(guild_id, member).await?;
                }
            }
            Event::UnavailableGuild(event) => self.store_unavailable_guild(event.id).await?,
            Event::UserUpdate(event) => self.store_current_user(event).await?,
            Event::VoiceServerUpdate(_) => {}
            Event::VoiceStateUpdate(event) => {
                if let Some(channel_id) = event.channel_id {
                    self.store_voice_state(channel_id, event).await?
                } else if let Some(guild_id) = event.guild_id {
                    self.delete_voice_state(guild_id, event.user_id).await?
                }
            }
            Event::WebhooksUpdate(_) => {}
        }

        Ok(())
    }

    async fn store_channel<'a>(&self, channel: &'a Channel) -> CacheResult<()> {
        if C::Channel::WANTED {
            let mut conn = self.connection().await?;

            let guild_id = channel.guild_id;
            let channel_id = channel.id;
            let key = RedisKey::Channel { id: channel_id };
            let channel = C::Channel::from_channel(channel);
            let bytes = channel.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;

            if let Some(guild_id) = guild_id {
                let key = RedisKey::GuildChannels { id: guild_id };
                conn.sadd(key, channel_id.get()).await?;
            }

            let key = RedisKey::Channels;
            conn.sadd(key, channel_id.get()).await?;
        }

        if let Some(ref member) = channel.member {
            if let (Some(guild_id), Some(member)) = (channel.guild_id, &member.member) {
                self.store_member(guild_id, member).await?;
            }

            if let Some(ref presence) = member.presence {
                self.store_presence(presence).await?;
            }
        }

        if let Some(ref users) = channel.recipients {
            self.store_users(users).await?;
        }

        Ok(())
    }

    async fn store_channels<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        channels: &'a [Channel],
    ) -> CacheResult<()> {
        if C::Channel::WANTED {
            let mut serializer = ChannelSerializer::<C>::default();

            let (channels, channel_ids): (Vec<_>, Vec<_>) = channels
                .iter()
                .map(|channel| {
                    let id = channel.id;
                    let key = RedisKey::Channel { id };
                    let channel = C::Channel::from_channel(channel);
                    let bytes = channel.serialize_with(&mut serializer).unwrap();

                    ((key, BytesRedisArgs(bytes)), id.get())
                })
                .unzip();

            if !channels.is_empty() {
                let mut conn = self.connection().await?;

                conn.mset(&channels).await?;

                let key = RedisKey::GuildChannels { id: guild_id };
                conn.sadd(key, channel_ids.as_slice()).await?;

                let key = RedisKey::Channels;
                conn.sadd(key, channel_ids).await?;
            }
        }

        let users = channels
            .iter()
            .filter_map(|channel| channel.recipients.as_ref())
            .flatten();

        self.store_users(users).await?;

        Ok(())
    }

    async fn store_current_user<'a>(&self, current_user: &'a CurrentUser) -> CacheResult<()> {
        if !C::CurrentUser::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::CurrentUser;
        let current_user = C::CurrentUser::from_current_user(current_user);
        let bytes = current_user.serialize().unwrap();

        conn.set(key, bytes.as_ref()).await?;

        Ok(())
    }

    async fn store_emojis<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        emojis: &'a [Emoji],
    ) -> CacheResult<()> {
        if !C::Emoji::WANTED {
            return Ok(());
        }

        let mut serializer = EmojiSerializer::<C>::default();

        let (emojis, emoji_ids): (Vec<_>, Vec<_>) = emojis
            .iter()
            .map(|emoji| {
                let id = emoji.id;
                let key = RedisKey::Emoji { id };
                let emoji = C::Emoji::from_emoji(emoji);
                let bytes = emoji.serialize_with(&mut serializer).unwrap();

                ((key, BytesRedisArgs(bytes)), id.get())
            })
            .unzip();

        if emojis.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        conn.mset(&emojis).await?;

        let key = RedisKey::GuildEmojis { id: guild_id };
        conn.sadd(key, emoji_ids.as_slice()).await?;

        let key = RedisKey::Emojis;
        conn.sadd(key, emoji_ids).await?;

        Ok(())
    }

    async fn store_guild<'a>(&self, guild: &'a Guild) -> CacheResult<()> {
        if C::Guild::WANTED {
            let mut conn = self.connection().await?;

            let guild_id = guild.id;
            let key = RedisKey::Guild { id: guild_id };
            let guild = C::Guild::from_guild(guild);
            let bytes = guild.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;

            let key = RedisKey::Guilds;
            conn.sadd(key, guild_id.get()).await?;

            let key = RedisKey::UnavailableGuilds;
            conn.srem(key, guild_id.get()).await?;
        }

        self.store_channels(guild.id, &guild.channels).await?;
        self.store_emojis(guild.id, &guild.emojis).await?;
        self.store_members(guild.id, &guild.members).await?;
        self.store_presences(guild.id, &guild.presences).await?;
        self.store_roles(guild.id, &guild.roles).await?;
        self.store_stickers(guild.id, &guild.stickers).await?;
        self.store_channels(guild.id, &guild.threads).await?;
        self.store_stage_instances(guild.id, &guild.stage_instances)
            .await?;
        self.store_voice_states(guild.id, &guild.voice_states)
            .await?;

        Ok(())
    }

    async fn store_integration<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        integration: &'a GuildIntegration,
    ) -> CacheResult<()> {
        if C::Integration::WANTED {
            let mut conn = self.connection().await?;

            let integration_id = integration.id;
            let key = RedisKey::Integration {
                guild: guild_id,
                id: integration_id,
            };
            let integration = C::Integration::from_integration(integration);
            let bytes = integration.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;

            let key = RedisKey::GuildIntegrations { id: guild_id };
            conn.sadd(key, integration_id.get()).await?;
        }

        if let Some(ref user) = integration.user {
            self.store_user(user).await?;
        }

        Ok(())
    }

    async fn store_member<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        member: &'a Member,
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let mut conn = self.connection().await?;

            let user_id = member.user.id;
            let key = RedisKey::Member {
                guild: guild_id,
                user: user_id,
            };
            let member = C::Member::from_member(guild_id, member);
            let bytes = member.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;

            let key = RedisKey::GuildMembers { id: guild_id };
            conn.sadd(key, user_id.get()).await?;

            let key = RedisKey::UserGuilds { id: user_id };
            conn.sadd(key, guild_id.get()).await?;
        }

        self.store_user(&member.user).await?;

        Ok(())
    }

    async fn store_member_update<'a>(&self, update: &'a MemberUpdate) -> CacheResult<()> {
        if C::Member::WANTED {
            let mut conn = self.connection().await?;

            let user_id = update.user.id;
            let key = RedisKey::Member {
                guild: update.guild_id,
                user: user_id,
            };
            if let Some(member) = C::Member::from_member_update(update) {
                let bytes = member.serialize().unwrap();
                conn.set(key, bytes.as_ref()).await?;
            }

            let key = RedisKey::GuildMembers {
                id: update.guild_id,
            };
            conn.sadd(key, user_id.get()).await?;

            let key = RedisKey::UserGuilds { id: user_id };
            conn.sadd(key, update.guild_id.get()).await?;
        }

        self.store_user(&update.user).await?;

        Ok(())
    }

    async fn store_members<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        members: &'a [Member],
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let mut serializer = MemberSerializer::<C>::default();

            let (member_tuples, user_ids): (Vec<_>, Vec<_>) = members
                .iter()
                .map(|member| {
                    let user_id = member.user.id;
                    let key = RedisKey::Member {
                        guild: guild_id,
                        user: user_id,
                    };
                    let member = C::Member::from_member(guild_id, member);
                    let bytes = member.serialize_with(&mut serializer).unwrap();

                    ((key, BytesRedisArgs(bytes)), user_id.get())
                })
                .unzip();

            if !member_tuples.is_empty() {
                let mut conn = self.connection().await?;

                conn.mset(&member_tuples).await?;

                let key = RedisKey::GuildMembers { id: guild_id };
                conn.sadd(key, user_ids.as_slice()).await?;

                for member in members {
                    let key = RedisKey::UserGuilds { id: member.user.id };
                    conn.sadd(key, guild_id.get()).await?;
                }
            }
        }

        let users = members.iter().map(|member| &member.user);
        self.store_users(users).await?;

        Ok(())
    }

    async fn store_message<'a>(&self, msg: &'a Message) -> CacheResult<()> {
        if C::Message::WANTED {
            let mut conn = self.connection().await?;

            let key = RedisKey::Message { id: msg.id };
            let msg = C::Message::from_message(msg);
            let bytes = msg.serialize().unwrap();

            if let Some(seconds) = C::Message::expire_seconds() {
                conn.set_ex(key, bytes.as_ref(), seconds).await?;
            } else {
                conn.set(key, bytes.as_ref()).await?;
            }
        }

        self.store_user(&msg.author).await?;

        if let (Some(guild_id), Some(member)) = (msg.guild_id, &msg.member) {
            self.store_partial_member(guild_id, member).await?;
        }

        if let Some(ref channel) = msg.thread {
            self.store_channel(channel).await?;
        }

        Ok(())
    }

    async fn store_message_update<'a>(&self, update: &'a MessageUpdate) -> CacheResult<()> {
        if C::Message::WANTED {
            if let Some(msg) = C::Message::from_message_update(update) {
                let mut conn = self.connection().await?;

                let key = RedisKey::Message { id: update.id };
                let bytes = msg.serialize().unwrap();

                if let Some(seconds) = C::Message::expire_seconds() {
                    conn.set_ex(key, bytes.as_ref(), seconds).await?;
                } else {
                    conn.set(key, bytes.as_ref()).await?;
                }
            }
        }

        if let Some(ref user) = update.author {
            self.store_user(user).await?;
        }

        Ok(())
    }

    async fn store_partial_guild<'a>(&self, guild: &'a PartialGuild) -> CacheResult<()> {
        if C::Guild::WANTED {
            let mut conn = self.connection().await?;

            let guild_id = guild.id;

            if let Some(guild) = C::Guild::from_partial_guild(guild) {
                let key = RedisKey::Guild { id: guild_id };
                let bytes = guild.serialize().unwrap();
                conn.set(key, bytes.as_ref()).await?;
            }

            let key = RedisKey::Guilds;
            conn.sadd(key, guild_id.get()).await?;

            let key = RedisKey::UnavailableGuilds;
            conn.srem(key, guild_id.get()).await?;
        }

        self.store_emojis(guild.id, &guild.emojis).await?;
        self.store_roles(guild.id, &guild.roles).await?;

        Ok(())
    }

    async fn store_partial_member<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        member: &'a PartialMember,
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            if let Some(ref user) = member.user {
                let mut conn = self.connection().await?;

                if let Some(member) = C::Member::from_partial_member(guild_id, member) {
                    let key = RedisKey::Member {
                        guild: guild_id,
                        user: user.id,
                    };
                    let bytes = member.serialize().unwrap();
                    conn.set(key, bytes.as_ref()).await?;
                }

                let key = RedisKey::GuildMembers { id: guild_id };
                conn.sadd(key, user.id.get()).await?;

                let key = RedisKey::UserGuilds { id: user.id };
                conn.sadd(key, guild_id.get()).await?;
            }
        }

        if let Some(ref user) = member.user {
            self.store_user(user).await?;
        }

        Ok(())
    }

    async fn store_partial_user<'a>(&self, user: &'a PartialUser) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let id = user.id;
        let key = RedisKey::User { id };

        if let Some(user) = C::User::from_partial_user(user) {
            let bytes = user.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;
        }

        let key = RedisKey::Users;
        conn.sadd(key, id.get()).await?;

        Ok(())
    }

    async fn store_presence<'a>(&self, presence: &'a Presence) -> CacheResult<()> {
        if C::Presence::WANTED {
            let mut conn = self.connection().await?;

            let guild_id = presence.guild_id;
            let user_id = presence.user.id();
            let key = RedisKey::Presence {
                guild: guild_id,
                user: user_id,
            };
            let presence = C::Presence::from_presence(presence);
            let bytes = presence.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;

            let key = RedisKey::GuildPresences { id: guild_id };
            conn.sadd(key, user_id.get()).await?;
        }

        if let UserOrId::User(ref user) = presence.user {
            self.store_user(user).await?;
        }

        Ok(())
    }

    async fn store_presences<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        presences: &'a [Presence],
    ) -> CacheResult<()> {
        if C::Presence::WANTED {
            let mut serializer = PresenceSerializer::<C>::default();

            let (presences, user_ids): (Vec<_>, Vec<_>) = presences
                .iter()
                .map(|presence| {
                    let guild_id = presence.guild_id;
                    let user_id = presence.user.id();
                    let key = RedisKey::Presence {
                        guild: guild_id,
                        user: user_id,
                    };
                    let presence = C::Presence::from_presence(presence);
                    let bytes = presence.serialize_with(&mut serializer).unwrap();

                    ((key, BytesRedisArgs(bytes)), user_id.get())
                })
                .unzip();

            if !presences.is_empty() {
                let mut conn = self.connection().await?;

                conn.mset(&presences).await?;

                let key = RedisKey::GuildPresences { id: guild_id };
                conn.sadd(key, user_ids.as_slice()).await?;
            }
        }

        let users = presences.iter().filter_map(|presence| match presence.user {
            UserOrId::User(ref user) => Some(user),
            UserOrId::UserId { .. } => None,
        });

        self.store_users(users).await?;

        Ok(())
    }

    async fn store_role<'a>(&self, guild_id: Id<GuildMarker>, role: &'a Role) -> CacheResult<()> {
        if !C::Role::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let id = role.id;
        let key = RedisKey::Role { id };
        let role = C::Role::from_role(role);
        let bytes = role.serialize().unwrap();
        conn.set(key, bytes.as_ref()).await?;

        let key = RedisKey::GuildRoles { id: guild_id };
        conn.sadd(key, id.get()).await?;

        let key = RedisKey::Roles;
        conn.sadd(key, id.get()).await?;

        Ok(())
    }

    async fn store_roles<'a, I>(&self, guild_id: Id<GuildMarker>, roles: I) -> CacheResult<()>
    where
        I: IntoIterator<Item = &'a Role>,
    {
        if !C::Role::WANTED {
            return Ok(());
        }

        let mut serializer = RoleSerializer::<C>::default();

        let (roles, role_ids): (Vec<_>, Vec<_>) = roles
            .into_iter()
            .map(|role| {
                let id = role.id;
                let key = RedisKey::Role { id };
                let role = C::Role::from_role(role);
                let bytes = role.serialize_with(&mut serializer).unwrap();

                ((key, BytesRedisArgs(bytes)), id.get())
            })
            .unzip();

        if roles.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        conn.mset(&roles).await?;

        let key = RedisKey::GuildRoles { id: guild_id };
        conn.sadd(key, role_ids.as_slice()).await?;

        let key = RedisKey::Roles;
        conn.sadd(key, role_ids).await?;

        Ok(())
    }

    async fn store_stage_instance<'a>(&self, stage_instance: &'a StageInstance) -> CacheResult<()> {
        if !C::StageInstance::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let stage_instance_id = stage_instance.id;
        let guild_id = stage_instance.guild_id;
        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        let stage_instance = C::StageInstance::from_stage_instance(stage_instance);
        let bytes = stage_instance.serialize().unwrap();
        conn.set(key, bytes.as_ref()).await?;

        let key = RedisKey::GuildStageInstances { id: guild_id };
        conn.sadd(key, stage_instance_id.get()).await?;

        let key = RedisKey::StageInstances;
        conn.sadd(key, stage_instance_id.get()).await?;

        Ok(())
    }

    async fn store_stage_instances<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        stage_instances: &'a [StageInstance],
    ) -> CacheResult<()> {
        if !C::StageInstance::WANTED {
            return Ok(());
        }

        let mut serializer = StageInstanceSerializer::<C>::default();

        let (stage_instances, stage_instance_ids): (Vec<_>, Vec<_>) = stage_instances
            .iter()
            .map(|stage_instance| {
                let id = stage_instance.id;
                let key = RedisKey::StageInstance { id };
                let stage_instance = C::StageInstance::from_stage_instance(stage_instance);
                let bytes = stage_instance.serialize_with(&mut serializer).unwrap();

                ((key, BytesRedisArgs(bytes)), id.get())
            })
            .unzip();

        if stage_instances.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        conn.mset(&stage_instances).await?;

        let key = RedisKey::GuildStageInstances { id: guild_id };
        conn.sadd(key, stage_instance_ids.as_slice()).await?;

        let key = RedisKey::StageInstances;
        conn.sadd(key, stage_instance_ids).await?;

        Ok(())
    }

    async fn store_stickers<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        stickers: &'a [Sticker],
    ) -> CacheResult<()> {
        if !C::Sticker::WANTED {
            return Ok(());
        }

        let mut serializer = StickerSerializer::<C>::default();

        let (stickers, sticker_ids): (Vec<_>, Vec<_>) = stickers
            .iter()
            .map(|sticker| {
                let id = sticker.id;
                let key = RedisKey::Sticker { id };
                let sticker = C::Sticker::from_sticker(sticker);
                let bytes = sticker.serialize_with(&mut serializer).unwrap();

                ((key, BytesRedisArgs(bytes)), id.get())
            })
            .unzip();

        if stickers.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        conn.mset(&stickers).await?;

        let key = RedisKey::GuildStickers { id: guild_id };
        conn.sadd(key, sticker_ids.as_slice()).await?;

        let key = RedisKey::Stickers;
        conn.sadd(key, sticker_ids).await?;

        Ok(())
    }

    async fn store_unavailable_guild(&self, guild_id: Id<GuildMarker>) -> CacheResult<()> {
        if !C::Guild::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::UnavailableGuilds;
        conn.sadd(key, guild_id.get()).await?;

        self.delete_guild(guild_id).await?;

        Ok(())
    }

    async fn store_unavailable_guilds(
        &self,
        unavailable_guilds: &[UnavailableGuild],
    ) -> CacheResult<()> {
        if !C::Guild::WANTED || unavailable_guilds.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let guild_ids: Vec<_> = unavailable_guilds
            .iter()
            .map(|guild| guild.id.get())
            .collect();

        let key = RedisKey::UnavailableGuilds;
        conn.sadd(key, guild_ids.as_slice()).await?;

        self.delete_guilds(&guild_ids).await?;

        Ok(())
    }

    async fn store_user<'a>(&self, user: &'a User) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let id = user.id;
        let key = RedisKey::User { id };
        let user = C::User::from_user(user);
        let bytes = user.serialize().unwrap();
        conn.set(key, bytes.as_ref()).await?;

        let key = RedisKey::Users;
        conn.sadd(key, id.get()).await?;

        Ok(())
    }

    async fn store_users<'a, I>(&self, users: I) -> CacheResult<()>
    where
        I: IntoIterator<Item = &'a User>,
    {
        if !C::User::WANTED {
            return Ok(());
        }

        let mut serializer = UserSerializer::<C>::default();

        let (users, user_ids): (Vec<_>, Vec<_>) = users
            .into_iter()
            .map(|user| {
                let id = user.id;
                let key = RedisKey::User { id };
                let user = C::User::from_user(user);
                let bytes = user.serialize_with(&mut serializer).unwrap();

                ((key, BytesRedisArgs(bytes)), id.get())
            })
            .unzip();

        if users.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        conn.mset(&users).await?;

        let key = RedisKey::Users;
        conn.sadd(key, user_ids).await?;

        Ok(())
    }

    async fn store_voice_state<'a>(
        &self,
        channel_id: Id<ChannelMarker>,
        voice_state: &'a VoiceState,
    ) -> CacheResult<()> {
        let Some(guild_id) = voice_state.guild_id else {
            return Ok(());
        };

        if C::VoiceState::WANTED {
            let mut conn = self.connection().await?;

            let user_id = voice_state.user_id;
            let key = RedisKey::VoiceState {
                guild: guild_id,
                user: user_id,
            };
            let voice_state = C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);
            let bytes = voice_state.serialize().unwrap();
            conn.set(key, bytes.as_ref()).await?;

            let key = RedisKey::GuildVoiceStates { id: guild_id };
            conn.sadd(key, user_id.get()).await?;
        }

        if let Some(ref member) = voice_state.member {
            self.store_member(guild_id, member).await?;
        }

        Ok(())
    }

    async fn store_voice_states<'a>(
        &self,
        guild_id: Id<GuildMarker>,
        voice_states: &'a [VoiceState],
    ) -> CacheResult<()> {
        if !C::VoiceState::WANTED {
            return Ok(());
        }

        let mut serializer = VoiceStateSerializer::<C>::default();

        let (voice_states, user_ids): (Vec<_>, Vec<_>) = voice_states
            .iter()
            .filter_map(|voice_state| {
                let channel_id = voice_state.channel_id?;
                let user_id = voice_state.user_id;
                let key = RedisKey::VoiceState {
                    guild: guild_id,
                    user: user_id,
                };
                let voice_state =
                    C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);
                let bytes = voice_state.serialize_with(&mut serializer).unwrap();

                Some(((key, BytesRedisArgs(bytes)), user_id.get()))
            })
            .unzip();

        if voice_states.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        conn.mset(&voice_states).await?;

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        conn.sadd(key, user_ids.as_slice()).await?;

        Ok(())
    }

    async fn delete_channel(
        &self,
        guild_id: Option<Id<GuildMarker>>,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<()> {
        if !C::Channel::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::Channel { id: channel_id };
        conn.del(key).await?;

        if let Some(guild_id) = guild_id {
            let key = RedisKey::GuildChannels { id: guild_id };
            conn.srem(key, channel_id.get()).await?;
        }

        let key = RedisKey::Channels;
        conn.srem(key, channel_id.get()).await?;

        Ok(())
    }

    async fn delete_guild(&self, guild_id: Id<GuildMarker>) -> CacheResult<()> {
        let mut conn = self.connection().await?;

        if C::Guild::WANTED {
            let key = RedisKey::Guild { id: guild_id };
            conn.del(key).await?;

            let key = RedisKey::Guilds;
            conn.srem(key, guild_id.get()).await?;
        }

        if C::Channel::WANTED {
            let key = RedisKey::GuildChannels { id: guild_id };
            let channel_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildChannels { id: guild_id };
            conn.del(key).await?;

            let key = RedisKey::Channels;
            conn.srem(key, channel_ids.as_slice()).await?;

            let channel_keys: Vec<_> = channel_ids
                .into_iter()
                .map(|channel_id| RedisKey::Channel {
                    id: Id::new(channel_id),
                })
                .collect();

            conn.del(channel_keys).await?;
        }

        if C::Emoji::WANTED {
            let key = RedisKey::GuildEmojis { id: guild_id };
            let emoji_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildEmojis { id: guild_id };
            conn.del(key).await?;

            let key = RedisKey::Emojis;
            conn.srem(key, emoji_ids.as_slice()).await?;

            let emoji_keys: Vec<_> = emoji_ids
                .into_iter()
                .map(|emoji_id| RedisKey::Emoji {
                    id: Id::new(emoji_id),
                })
                .collect();

            conn.del(emoji_keys).await?;
        }

        if C::Integration::WANTED {
            let key = RedisKey::GuildIntegrations { id: guild_id };
            let integration_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildIntegrations { id: guild_id };
            conn.del(key).await?;

            let integration_keys: Vec<_> = integration_ids
                .into_iter()
                .map(|integration_id| RedisKey::Integration {
                    guild: guild_id,
                    id: Id::new(integration_id),
                })
                .collect();

            conn.del(integration_keys).await?;
        }

        if C::Member::WANTED {
            let key = RedisKey::GuildMembers { id: guild_id };
            let user_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildMembers { id: guild_id };
            conn.del(key).await?;

            let member_keys: Vec<_> = user_ids
                .iter()
                .map(|&user_id| RedisKey::Member {
                    guild: guild_id,
                    user: Id::new(user_id),
                })
                .collect();

            conn.del(member_keys).await?;

            for &user_id in user_ids.iter() {
                let key = RedisKey::UserGuilds {
                    id: Id::new(user_id),
                };
                conn.srem(key, guild_id.get()).await?;
            }

            if C::User::WANTED {
                for user_id in user_ids {
                    let user_id = Id::new(user_id);

                    let key = RedisKey::UserGuilds { id: user_id };
                    let common_guild_count: usize = conn.scard(key).await?;

                    if common_guild_count == 0 {
                        let key = RedisKey::User { id: user_id };
                        conn.del(key).await?;

                        let key = RedisKey::Users;
                        conn.srem(key, user_id.get()).await?;
                    }
                }
            }
        }

        if C::Presence::WANTED {
            let key = RedisKey::GuildPresences { id: guild_id };
            let user_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildPresences { id: guild_id };
            conn.del(key).await?;

            let presence_keys: Vec<_> = user_ids
                .into_iter()
                .map(|user_id| RedisKey::Presence {
                    guild: guild_id,
                    user: Id::new(user_id),
                })
                .collect();

            conn.del(presence_keys).await?;
        }

        if C::Role::WANTED {
            let key = RedisKey::GuildRoles { id: guild_id };
            let role_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildRoles { id: guild_id };
            conn.del(key).await?;

            let key = RedisKey::Roles;
            conn.srem(key, role_ids.as_slice()).await?;

            let role_keys: Vec<_> = role_ids
                .into_iter()
                .map(|role_id| RedisKey::Role {
                    id: Id::new(role_id),
                })
                .collect();

            conn.del(role_keys).await?;
        }

        if C::StageInstance::WANTED {
            let key = RedisKey::GuildStageInstances { id: guild_id };
            let stage_instance_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildStageInstances { id: guild_id };
            conn.del(key).await?;

            let key = RedisKey::StageInstances;
            conn.srem(key, stage_instance_ids.as_slice()).await?;

            let stage_instance_keys: Vec<_> = stage_instance_ids
                .into_iter()
                .map(|stage_instance_id| RedisKey::StageInstance {
                    id: Id::new(stage_instance_id),
                })
                .collect();

            conn.del(stage_instance_keys).await?;
        }

        if C::Sticker::WANTED {
            let key = RedisKey::GuildStickers { id: guild_id };
            let sticker_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildStickers { id: guild_id };
            conn.del(key).await?;

            let key = RedisKey::Stickers;
            conn.srem(key, sticker_ids.as_slice()).await?;

            let sticker_keys: Vec<_> = sticker_ids
                .into_iter()
                .map(|sticker_id| RedisKey::Sticker {
                    id: Id::new(sticker_id),
                })
                .collect();

            conn.del(sticker_keys).await?;
        }

        if C::VoiceState::WANTED {
            let key = RedisKey::GuildVoiceStates { id: guild_id };
            let user_ids: Vec<_> = conn.smembers(key).await?;

            let key = RedisKey::GuildVoiceStates { id: guild_id };
            conn.del(key).await?;

            let voice_state_keys: Vec<_> = user_ids
                .into_iter()
                .map(|user_id| RedisKey::VoiceState {
                    guild: guild_id,
                    user: Id::new(user_id),
                })
                .collect();

            conn.del(voice_state_keys).await?;
        }

        Ok(())
    }

    async fn delete_guilds(&self, guild_ids: &[u64]) -> CacheResult<()> {
        todo!()
    }

    async fn delete_integration(
        &self,
        guild_id: Id<GuildMarker>,
        integration_id: Id<IntegrationMarker>,
    ) -> CacheResult<()> {
        if !C::Integration::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::Integration {
            guild: guild_id,
            id: integration_id,
        };
        conn.del(key).await?;

        let key = RedisKey::GuildIntegrations { id: guild_id };
        conn.srem(key, integration_id.get()).await?;

        Ok(())
    }

    async fn delete_member(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<()> {
        if !C::Member::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::Member {
            guild: guild_id,
            user: user_id,
        };
        conn.del(key).await?;

        let key = RedisKey::GuildMembers { id: guild_id };
        conn.srem(key, user_id.get()).await?;

        let key = RedisKey::UserGuilds { id: user_id };
        conn.srem(key, guild_id.get()).await?;

        if C::User::WANTED {
            let key = RedisKey::UserGuilds { id: user_id };
            let common_guild_count: usize = conn.scard(key).await?;

            if common_guild_count == 0 {
                let key = RedisKey::User { id: user_id };
                conn.del(key).await?;

                let key = RedisKey::Users;
                conn.srem(key, user_id.get()).await?;
            }
        }

        Ok(())
    }

    async fn delete_message(&self, msg_id: Id<MessageMarker>) -> CacheResult<()> {
        if !C::Message::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::Message { id: msg_id };
        conn.del(key).await?;

        Ok(())
    }

    async fn delete_messages(&self, msg_ids: &[Id<MessageMarker>]) -> CacheResult<()> {
        if !C::Message::WANTED || msg_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let keys: Vec<_> = msg_ids
            .iter()
            .copied()
            .map(|id| RedisKey::Message { id })
            .collect();

        conn.del(keys).await?;

        Ok(())
    }

    async fn delete_role(
        &self,
        guild_id: Id<GuildMarker>,
        role_id: Id<RoleMarker>,
    ) -> CacheResult<()> {
        if !C::Role::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::Role { id: role_id };
        conn.del(key).await?;

        let key = RedisKey::GuildRoles { id: guild_id };
        conn.srem(key, role_id.get()).await?;

        let key = RedisKey::Roles;
        conn.srem(key, role_id.get()).await?;

        Ok(())
    }

    async fn delete_stage_instance(
        &self,
        guild_id: Id<GuildMarker>,
        stage_instance_id: Id<StageMarker>,
    ) -> CacheResult<()> {
        if !C::StageInstance::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        conn.del(key).await?;

        let key = RedisKey::GuildStageInstances { id: guild_id };
        conn.srem(key, stage_instance_id.get()).await?;

        let key = RedisKey::StageInstances;
        conn.srem(key, stage_instance_id.get()).await?;

        Ok(())
    }

    async fn delete_voice_state(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<()> {
        if !C::VoiceState::WANTED {
            return Ok(());
        }

        let mut conn = self.connection().await?;

        let key = RedisKey::VoiceState {
            guild: guild_id,
            user: user_id,
        };
        conn.del(key).await?;

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        conn.srem(key, user_id.get()).await?;

        Ok(())
    }

    pub async fn channel(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<Option<CachedValue<C::Channel<'static>>>> {
        self.get_single(channel_id).await
    }

    pub async fn current_user<const CURRENT_USER_SCRATCH: usize>(
        &self,
    ) -> CacheResult<Option<CachedValue<C::CurrentUser<'static>>>> {
        self.get_single(RedisKey::CurrentUser).await
    }

    pub async fn emoji<const EMOJI_SCRATCH: usize>(
        &self,
        emoji_id: Id<EmojiMarker>,
    ) -> CacheResult<Option<CachedValue<C::Emoji<'static>>>> {
        self.get_single(emoji_id).await
    }

    pub async fn guild(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<Option<CachedValue<C::Guild<'static>>>> {
        self.get_single(guild_id).await
    }

    pub async fn guild_emojis(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<Vec<Id<EmojiMarker>>> {
        let mut conn = self.connection().await?;
        let key = RedisKey::GuildEmojis { id: guild_id };
        let emojis_raw: Vec<u64> = conn.smembers(key).await?;
        let emojis = emojis_raw.into_iter().map(Id::new).collect();

        Ok(emojis)
    }

    pub async fn guild_roles(&self, guild_id: Id<GuildMarker>) -> CacheResult<Vec<Id<RoleMarker>>> {
        let mut conn = self.connection().await?;
        let key = RedisKey::GuildRoles { id: guild_id };
        let roles_raw: Vec<u64> = conn.smembers(key).await?;
        let roles = roles_raw.into_iter().map(Id::new).collect();

        Ok(roles)
    }

    pub async fn guild_stickers(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<Vec<Id<StickerMarker>>> {
        let mut conn = self.connection().await?;
        let key = RedisKey::GuildStickers { id: guild_id };
        let stickers_raw: Vec<u64> = conn.smembers(key).await?;
        let stickers = stickers_raw.into_iter().map(Id::new).collect();

        Ok(stickers)
    }

    pub async fn member(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedValue<C::Member<'static>>>> {
        let key = RedisKey::Member {
            guild: guild_id,
            user: user_id,
        };

        self.get_single(key).await
    }

    pub async fn message(
        &self,
        msg_id: Id<MessageMarker>,
    ) -> CacheResult<Option<CachedValue<C::Message<'static>>>> {
        self.get_single(msg_id).await
    }

    pub async fn presence(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedValue<C::Presence<'static>>>> {
        let key = RedisKey::Presence {
            guild: guild_id,
            user: user_id,
        };

        self.get_single(key).await
    }

    pub async fn role(
        &self,
        role_id: Id<RoleMarker>,
    ) -> CacheResult<Option<CachedValue<C::Role<'static>>>> {
        self.get_single(role_id).await
    }

    pub async fn sticker(
        &self,
        sticker_id: Id<StickerMarker>,
    ) -> CacheResult<Option<CachedValue<C::Sticker<'static>>>> {
        self.get_single(sticker_id).await
    }

    pub async fn unavailable_guilds(&self) -> CacheResult<Vec<Id<GuildMarker>>> {
        todo!()
    }

    pub async fn unavailable_guilds_count(&self) -> CacheResult<usize> {
        todo!()
    }

    pub async fn user(
        &self,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedValue<C::User<'static>>>> {
        self.get_single(user_id).await
    }

    pub async fn voice_state(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedValue<C::VoiceState<'static>>>> {
        let key = RedisKey::VoiceState {
            guild: guild_id,
            user: user_id,
        };

        self.get_single(key).await
    }

    async fn get_single<K, V>(&self, key: K) -> CacheResult<Option<CachedValue<V>>>
    where
        RedisKey: From<K>,
        V: Cacheable,
    {
        let mut conn = self.connection().await?;
        let bytes: Vec<u8> = conn.get(RedisKey::from(key)).await?;

        if bytes.is_empty() {
            return Ok(None);
        }

        CachedValue::new(bytes).map(Some)
    }
}
