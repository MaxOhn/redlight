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
    redis::{aio::Connection, AsyncCommands, FromRedisValue, Pipeline},
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

    async fn query_pipe<T: FromRedisValue>(
        pipe: &Pipeline,
        conn: &mut Connection,
    ) -> CacheResult<T> {
        pipe.query_async(conn).await.map_err(CacheError::Redis)
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
    pub async fn update(&self, event: &Event) -> CacheResult<()> {
        let start = std::time::Instant::now();

        let mut pipe = Pipeline::new();

        match event {
            Event::AutoModerationActionExecution(_) => {}
            Event::AutoModerationRuleCreate(_) => {}
            Event::AutoModerationRuleDelete(_) => {}
            Event::AutoModerationRuleUpdate(_) => {}
            Event::BanAdd(event) => self.store_user(&mut pipe, &event.user),
            Event::BanRemove(event) => self.store_user(&mut pipe, &event.user),
            Event::ChannelCreate(event) => self.store_channel(&mut pipe, event),
            Event::ChannelDelete(event) => self.delete_channel(&mut pipe, event.guild_id, event.id),
            Event::ChannelPinsUpdate(_) => {}
            Event::ChannelUpdate(event) => self.store_channel(&mut pipe, event),
            Event::CommandPermissionsUpdate(_) => {}
            Event::GatewayClose(_) => {}
            Event::GatewayHeartbeat(_) => {}
            Event::GatewayHeartbeatAck => {}
            Event::GatewayHello(_) => {}
            Event::GatewayInvalidateSession(_) => {}
            Event::GatewayReconnect => {}
            Event::GiftCodeUpdate => {}
            Event::GuildAuditLogEntryCreate(_) => {}
            Event::GuildCreate(event) => self.store_guild(&mut pipe, event),
            Event::GuildDelete(event) => {
                if event.unavailable {
                    self.store_unavailable_guild(&mut pipe, event.id).await?
                } else {
                    self.delete_guild(&mut pipe, event.id).await?
                }
            }
            Event::GuildEmojisUpdate(event) => {
                self.store_emojis(&mut pipe, event.guild_id, &event.emojis)
            }
            Event::GuildIntegrationsUpdate(_) => {}
            Event::GuildScheduledEventCreate(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(&mut pipe, user);
                }
            }
            Event::GuildScheduledEventDelete(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(&mut pipe, user);
                }
            }
            Event::GuildScheduledEventUpdate(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(&mut pipe, user);
                }
            }
            Event::GuildScheduledEventUserAdd(_) => {}
            Event::GuildScheduledEventUserRemove(_) => {}
            Event::GuildStickersUpdate(event) => {
                self.store_stickers(&mut pipe, event.guild_id, &event.stickers)
            }
            Event::GuildUpdate(event) => self.store_partial_guild(&mut pipe, event),
            Event::IntegrationCreate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(&mut pipe, guild_id, event);
                }
            }
            Event::IntegrationDelete(event) => {
                self.delete_integration(&mut pipe, event.guild_id, event.id)
            }
            Event::IntegrationUpdate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(&mut pipe, guild_id, event);
                }
            }
            Event::InteractionCreate(event) => {
                if let Some(ref channel) = event.channel {
                    self.store_channel(&mut pipe, channel);
                }

                if let Some(InteractionData::ApplicationCommand(ref data)) = event.data {
                    if let Some(ref resolved) = data.resolved {
                        if let Some(guild_id) = event.guild_id {
                            let roles = resolved.roles.values();
                            self.store_roles(&mut pipe, guild_id, roles);
                        }

                        let users = resolved.users.values();
                        self.store_users(&mut pipe, users);
                    }
                }

                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_partial_member(&mut pipe, guild_id, member);
                }

                if let Some(ref msg) = event.message {
                    self.store_message(&mut pipe, msg);
                }

                if let Some(ref user) = event.user {
                    self.store_user(&mut pipe, user);
                }
            }
            Event::InviteCreate(event) => {
                if let Some(ref user) = event.inviter {
                    self.store_user(&mut pipe, user);
                }

                if let Some(ref user) = event.target_user {
                    self.store_partial_user(&mut pipe, user);
                }
            }
            Event::InviteDelete(_) => {}
            Event::MemberAdd(event) => self.store_member(&mut pipe, event.guild_id, &event.member),
            Event::MemberRemove(event) => {
                self.delete_member(&mut pipe, event.guild_id, event.user.id)
                    .await?
            }
            Event::MemberUpdate(event) => self.store_member_update(&mut pipe, event),
            Event::MemberChunk(event) => {
                self.store_members(&mut pipe, event.guild_id, &event.members);
                self.store_presences(&mut pipe, event.guild_id, &event.presences);
            }
            Event::MessageCreate(event) => self.store_message(&mut pipe, event),
            Event::MessageDelete(event) => self.delete_message(&mut pipe, event.id),
            Event::MessageDeleteBulk(event) => self.delete_messages(&mut pipe, &event.ids),
            Event::MessageUpdate(event) => self.store_message_update(&mut pipe, event),
            Event::PresenceUpdate(event) => self.store_presence(&mut pipe, event),
            Event::PresencesReplace => {}
            Event::ReactionAdd(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member);
                }
            }
            Event::ReactionRemove(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member);
                }
            }
            Event::ReactionRemoveAll(_) => {}
            Event::ReactionRemoveEmoji(_) => {}
            Event::Ready(event) => {
                self.store_unavailable_guilds(&mut pipe, &event.guilds)
                    .await?;
                self.store_current_user(&mut pipe, &event.user);
            }
            Event::Resumed => {}
            Event::RoleCreate(event) => self.store_role(&mut pipe, event.guild_id, &event.role),
            Event::RoleDelete(event) => self.delete_role(&mut pipe, event.guild_id, event.role_id),
            Event::RoleUpdate(event) => self.store_role(&mut pipe, event.guild_id, &event.role),
            Event::StageInstanceCreate(event) => self.store_stage_instance(&mut pipe, event),
            Event::StageInstanceDelete(event) => {
                self.delete_stage_instance(&mut pipe, event.guild_id, event.id)
            }
            Event::StageInstanceUpdate(event) => self.store_stage_instance(&mut pipe, event),
            Event::ThreadCreate(event) => self.store_channel(&mut pipe, event),
            Event::ThreadDelete(event) => {
                self.delete_channel(&mut pipe, Some(event.guild_id), event.id)
            }
            Event::ThreadListSync(event) => {
                self.store_channels(&mut pipe, event.guild_id, &event.threads)
            }
            Event::ThreadMemberUpdate(event) => {
                if let Some(ref presence) = event.presence {
                    self.store_presence(&mut pipe, presence);

                    if let Some(ref member) = event.member {
                        self.store_member(&mut pipe, presence.guild_id, member);
                    }
                }
            }
            Event::ThreadMembersUpdate(_) => {}
            Event::ThreadUpdate(event) => self.store_channel(&mut pipe, event),
            Event::TypingStart(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member);
                }
            }
            Event::UnavailableGuild(event) => {
                self.store_unavailable_guild(&mut pipe, event.id).await?
            }
            Event::UserUpdate(event) => self.store_current_user(&mut pipe, event),
            Event::VoiceServerUpdate(_) => {}
            Event::VoiceStateUpdate(event) => {
                if let Some(channel_id) = event.channel_id {
                    self.store_voice_state(&mut pipe, channel_id, event);
                } else if let Some(guild_id) = event.guild_id {
                    self.delete_voice_state(&mut pipe, guild_id, event.user_id);
                }
            }
            Event::WebhooksUpdate(_) => {}
        };

        if pipe.cmd_iter().next().is_some() {
            let mut conn = self.connection().await?;
            Self::query_pipe::<()>(&pipe, &mut conn).await?;
        }

        let elapsed = start.elapsed();
        println!("{:?}: {elapsed:.2?}", event.kind());

        Ok(())
    }

    fn store_channel(&self, pipe: &mut Pipeline, channel: &Channel) {
        if C::Channel::WANTED {
            let guild_id = channel.guild_id;
            let channel_id = channel.id;
            let key = RedisKey::Channel { id: channel_id };
            let channel = C::Channel::from_channel(channel);
            let bytes = channel.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();

            if let Some(guild_id) = guild_id {
                let key = RedisKey::GuildChannels { id: guild_id };
                pipe.sadd(key, channel_id.get()).ignore();
            }

            let key = RedisKey::Channels;
            pipe.sadd(key, channel_id.get()).ignore();
        }

        if let Some(ref member) = channel.member {
            if let (Some(guild_id), Some(member)) = (channel.guild_id, &member.member) {
                self.store_member(pipe, guild_id, member);
            }

            if let Some(ref presence) = member.presence {
                self.store_presence(pipe, presence);
            }
        }

        if let Some(ref users) = channel.recipients {
            self.store_users(pipe, users);
        }
    }

    fn store_channels(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, channels: &[Channel]) {
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
                pipe.mset(&channels).ignore();

                let key = RedisKey::GuildChannels { id: guild_id };
                pipe.sadd(key, channel_ids.as_slice()).ignore();

                let key = RedisKey::Channels;
                pipe.sadd(key, channel_ids).ignore();
            }
        }

        let users = channels
            .iter()
            .filter_map(|channel| channel.recipients.as_ref())
            .flatten();

        self.store_users(pipe, users);
    }

    fn store_current_user(&self, pipe: &mut Pipeline, current_user: &CurrentUser) {
        if !C::CurrentUser::WANTED {
            return;
        }

        let key = RedisKey::CurrentUser;
        let current_user = C::CurrentUser::from_current_user(current_user);
        let bytes = current_user.serialize().unwrap();

        pipe.set(key, bytes.as_ref()).ignore();
    }

    fn store_emojis(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, emojis: &[Emoji]) {
        if !C::Emoji::WANTED {
            return;
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
            return;
        }

        pipe.mset(&emojis).ignore();

        let key = RedisKey::GuildEmojis { id: guild_id };
        pipe.sadd(key, emoji_ids.as_slice()).ignore();

        let key = RedisKey::Emojis;
        pipe.sadd(key, emoji_ids).ignore();
    }

    pub(crate) fn store_guild(&self, pipe: &mut Pipeline, guild: &Guild) {
        if C::Guild::WANTED {
            let guild_id = guild.id;
            let key = RedisKey::Guild { id: guild_id };
            let guild = C::Guild::from_guild(guild);
            let bytes = guild.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();

            let key = RedisKey::Guilds;
            pipe.sadd(key, guild_id.get()).ignore();

            let key = RedisKey::UnavailableGuilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        self.store_channels(pipe, guild.id, &guild.channels);
        self.store_emojis(pipe, guild.id, &guild.emojis);
        self.store_members(pipe, guild.id, &guild.members);
        self.store_presences(pipe, guild.id, &guild.presences);
        self.store_roles(pipe, guild.id, &guild.roles);
        self.store_stickers(pipe, guild.id, &guild.stickers);
        self.store_channels(pipe, guild.id, &guild.threads);
        self.store_stage_instances(pipe, guild.id, &guild.stage_instances);
        self.store_voice_states(pipe, guild.id, &guild.voice_states);
    }

    fn store_integration(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        integration: &GuildIntegration,
    ) {
        if C::Integration::WANTED {
            let integration_id = integration.id;
            let key = RedisKey::Integration {
                guild: guild_id,
                id: integration_id,
            };
            let integration = C::Integration::from_integration(integration);
            let bytes = integration.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();

            let key = RedisKey::GuildIntegrations { id: guild_id };
            pipe.sadd(key, integration_id.get()).ignore();
        }

        if let Some(ref user) = integration.user {
            self.store_user(pipe, user);
        }
    }

    fn store_member(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, member: &Member) {
        if C::Member::WANTED {
            let user_id = member.user.id;
            let key = RedisKey::Member {
                guild: guild_id,
                user: user_id,
            };
            let member = C::Member::from_member(guild_id, member);
            let bytes = member.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();

            let key = RedisKey::GuildMembers { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();

            if C::User::WANTED {
                let key = RedisKey::UserGuilds { id: user_id };
                pipe.sadd(key, guild_id.get()).ignore();
            }
        }

        self.store_user(pipe, &member.user);
    }

    fn store_member_update(&self, pipe: &mut Pipeline, update: &MemberUpdate) {
        if C::Member::WANTED {
            let user_id = update.user.id;
            let key = RedisKey::Member {
                guild: update.guild_id,
                user: user_id,
            };
            if let Some(member) = C::Member::from_member_update(update) {
                let bytes = member.serialize().unwrap();
                pipe.set(key, bytes.as_ref()).ignore();
            }

            let key = RedisKey::GuildMembers {
                id: update.guild_id,
            };
            pipe.sadd(key, user_id.get()).ignore();

            if C::User::WANTED {
                let key = RedisKey::UserGuilds { id: user_id };
                pipe.sadd(key, update.guild_id.get()).ignore();
            }
        }

        self.store_user(pipe, &update.user);
    }

    fn store_members(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, members: &[Member]) {
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
                pipe.mset(&member_tuples).ignore();

                let key = RedisKey::GuildMembers { id: guild_id };
                pipe.sadd(key, user_ids.as_slice()).ignore();

                if C::User::WANTED {
                    for member in members {
                        let key = RedisKey::UserGuilds { id: member.user.id };
                        pipe.sadd(key, guild_id.get()).ignore();
                    }
                }
            }
        }

        let users = members.iter().map(|member| &member.user);
        self.store_users(pipe, users);
    }

    fn store_message(&self, pipe: &mut Pipeline, msg: &Message) {
        if C::Message::WANTED {
            let key = RedisKey::Message { id: msg.id };
            let msg = C::Message::from_message(msg);
            let bytes = msg.serialize().unwrap();

            if let Some(seconds) = C::Message::expire_seconds() {
                pipe.set_ex(key, bytes.as_ref(), seconds).ignore();
            } else {
                pipe.set(key, bytes.as_ref()).ignore();
            }
        }

        self.store_user(pipe, &msg.author);

        if let (Some(guild_id), Some(member)) = (msg.guild_id, &msg.member) {
            self.store_partial_member(pipe, guild_id, member);
        }

        if let Some(ref channel) = msg.thread {
            self.store_channel(pipe, channel);
        }
    }

    fn store_message_update(&self, pipe: &mut Pipeline, update: &MessageUpdate) {
        if C::Message::WANTED {
            if let Some(msg) = C::Message::from_message_update(update) {
                let key = RedisKey::Message { id: update.id };
                let bytes = msg.serialize().unwrap();

                if let Some(seconds) = C::Message::expire_seconds() {
                    pipe.set_ex(key, bytes.as_ref(), seconds).ignore();
                } else {
                    pipe.set(key, bytes.as_ref()).ignore();
                }
            }
        }

        if let Some(ref user) = update.author {
            self.store_user(pipe, user);
        }
    }

    pub(crate) fn store_partial_guild(&self, pipe: &mut Pipeline, guild: &PartialGuild) {
        if C::Guild::WANTED {
            let guild_id = guild.id;

            if let Some(guild) = C::Guild::from_partial_guild(guild) {
                let key = RedisKey::Guild { id: guild_id };
                let bytes = guild.serialize().unwrap();
                pipe.set(key, bytes.as_ref()).ignore();
            }

            let key = RedisKey::Guilds;
            pipe.sadd(key, guild_id.get()).ignore();

            let key = RedisKey::UnavailableGuilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        self.store_emojis(pipe, guild.id, &guild.emojis);
        self.store_roles(pipe, guild.id, &guild.roles);
    }

    fn store_partial_member(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        member: &PartialMember,
    ) {
        if C::Member::WANTED {
            if let Some(ref user) = member.user {
                if let Some(member) = C::Member::from_partial_member(guild_id, member) {
                    let key = RedisKey::Member {
                        guild: guild_id,
                        user: user.id,
                    };
                    let bytes = member.serialize().unwrap();
                    pipe.set(key, bytes.as_ref()).ignore();
                }

                let key = RedisKey::GuildMembers { id: guild_id };
                pipe.sadd(key, user.id.get()).ignore();

                if C::User::WANTED {
                    let key = RedisKey::UserGuilds { id: user.id };
                    pipe.sadd(key, guild_id.get()).ignore();
                }
            }
        }

        if let Some(ref user) = member.user {
            self.store_user(pipe, user);
        }
    }

    fn store_partial_user(&self, pipe: &mut Pipeline, user: &PartialUser) {
        if !C::User::WANTED {
            return;
        }

        let id = user.id;
        let key = RedisKey::User { id };

        if let Some(user) = C::User::from_partial_user(user) {
            let bytes = user.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();
        }

        let key = RedisKey::Users;
        pipe.sadd(key, id.get()).ignore();
    }

    fn store_presence(&self, pipe: &mut Pipeline, presence: &Presence) {
        if C::Presence::WANTED {
            let guild_id = presence.guild_id;
            let user_id = presence.user.id();
            let key = RedisKey::Presence {
                guild: guild_id,
                user: user_id,
            };
            let presence = C::Presence::from_presence(presence);
            let bytes = presence.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();

            let key = RedisKey::GuildPresences { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if let UserOrId::User(ref user) = presence.user {
            self.store_user(pipe, user);
        }
    }

    fn store_presences(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        presences: &[Presence],
    ) {
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
                pipe.mset(&presences).ignore();

                let key = RedisKey::GuildPresences { id: guild_id };
                pipe.sadd(key, user_ids.as_slice()).ignore();
            }
        }

        let users = presences.iter().filter_map(|presence| match presence.user {
            UserOrId::User(ref user) => Some(user),
            UserOrId::UserId { .. } => None,
        });

        self.store_users(pipe, users);
    }

    fn store_role(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, role: &Role) {
        if !C::Role::WANTED {
            return;
        }

        let id = role.id;
        let key = RedisKey::Role { id };
        let role = C::Role::from_role(role);
        let bytes = role.serialize().unwrap();
        pipe.set(key, bytes.as_ref()).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, id.get()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, id.get()).ignore();
    }

    fn store_roles<'a, I>(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, roles: I)
    where
        I: IntoIterator<Item = &'a Role>,
    {
        if !C::Role::WANTED {
            return;
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
            return;
        }

        pipe.mset(&roles).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, role_ids.as_slice()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, role_ids).ignore();
    }

    fn store_stage_instance(&self, pipe: &mut Pipeline, stage_instance: &StageInstance) {
        if !C::StageInstance::WANTED {
            return;
        }

        let stage_instance_id = stage_instance.id;
        let guild_id = stage_instance.guild_id;
        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        let stage_instance = C::StageInstance::from_stage_instance(stage_instance);
        let bytes = stage_instance.serialize().unwrap();
        pipe.set(key, bytes.as_ref()).ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_id.get()).ignore();

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_id.get()).ignore();
    }

    fn store_stage_instances(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        stage_instances: &[StageInstance],
    ) {
        if !C::StageInstance::WANTED {
            return;
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
            return;
        }

        pipe.mset(&stage_instances).ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_ids.as_slice()).ignore();

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_ids).ignore();
    }

    fn store_stickers(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, stickers: &[Sticker]) {
        if !C::Sticker::WANTED {
            return;
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
            return;
        }

        pipe.mset(&stickers).ignore();

        let key = RedisKey::GuildStickers { id: guild_id };
        pipe.sadd(key, sticker_ids.as_slice()).ignore();

        let key = RedisKey::Stickers;
        pipe.sadd(key, sticker_ids).ignore();
    }

    async fn store_unavailable_guild(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        self.delete_guild(pipe, guild_id).await?;

        let key = RedisKey::UnavailableGuilds;
        pipe.sadd(key, guild_id.get()).ignore();

        Ok(())
    }

    async fn store_unavailable_guilds(
        &self,
        pipe: &mut Pipeline,
        unavailable_guilds: &[UnavailableGuild],
    ) -> CacheResult<()> {
        let guild_ids: Vec<_> = unavailable_guilds
            .iter()
            .map(|guild| guild.id.get())
            .collect();

        self.delete_guilds(pipe, &guild_ids).await?;

        let key = RedisKey::UnavailableGuilds;
        pipe.sadd(key, guild_ids.as_slice()).ignore();

        Ok(())
    }

    pub(crate) fn store_user(&self, pipe: &mut Pipeline, user: &User) {
        if !C::User::WANTED {
            return;
        }

        let id = user.id;
        let key = RedisKey::User { id };
        let user = C::User::from_user(user);
        let bytes = user.serialize().unwrap();
        pipe.set(key, bytes.as_ref()).ignore();

        let key = RedisKey::Users;
        pipe.sadd(key, id.get()).ignore();
    }

    fn store_users<'a, I>(&self, pipe: &mut Pipeline, users: I)
    where
        I: IntoIterator<Item = &'a User>,
    {
        if !C::User::WANTED {
            return;
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
            return;
        }

        pipe.mset(&users).ignore();

        let key = RedisKey::Users;
        pipe.sadd(key, user_ids).ignore();
    }

    fn store_voice_state(
        &self,
        pipe: &mut Pipeline,
        channel_id: Id<ChannelMarker>,
        voice_state: &VoiceState,
    ) {
        let Some(guild_id) = voice_state.guild_id else {
            return;
        };

        if C::VoiceState::WANTED {
            let user_id = voice_state.user_id;
            let key = RedisKey::VoiceState {
                guild: guild_id,
                user: user_id,
            };
            let voice_state = C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);
            let bytes = voice_state.serialize().unwrap();
            pipe.set(key, bytes.as_ref()).ignore();

            let key = RedisKey::GuildVoiceStates { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if let Some(ref member) = voice_state.member {
            self.store_member(pipe, guild_id, member);
        }
    }

    fn store_voice_states(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        voice_states: &[VoiceState],
    ) {
        if !C::VoiceState::WANTED {
            return;
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
            return;
        }

        pipe.mset(&voice_states).ignore();

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.sadd(key, user_ids.as_slice()).ignore();
    }

    fn delete_channel(
        &self,
        pipe: &mut Pipeline,
        guild_id: Option<Id<GuildMarker>>,
        channel_id: Id<ChannelMarker>,
    ) {
        if !C::Channel::WANTED {
            return;
        }

        let key = RedisKey::Channel { id: channel_id };
        pipe.del(key).ignore();

        if let Some(guild_id) = guild_id {
            let key = RedisKey::GuildChannels { id: guild_id };
            pipe.srem(key, channel_id.get()).ignore();
        }

        let key = RedisKey::Channels;
        pipe.srem(key, channel_id.get()).ignore();
    }

    async fn delete_guild(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        debug_assert!(pipe.cmd_iter().next().is_none());

        if C::Channel::WANTED {
            let key = RedisKey::GuildChannels { id: guild_id };
            pipe.smembers(key);
        }

        if C::Emoji::WANTED {
            let key = RedisKey::GuildEmojis { id: guild_id };
            pipe.smembers(key);
        }

        if C::Integration::WANTED {
            let key = RedisKey::GuildIntegrations { id: guild_id };
            pipe.smembers(key);
        }

        if C::Member::WANTED {
            let key = RedisKey::GuildMembers { id: guild_id };
            pipe.smembers(key);
        }

        if C::Presence::WANTED {
            let key = RedisKey::GuildPresences { id: guild_id };
            pipe.smembers(key);
        }

        if C::Role::WANTED {
            let key = RedisKey::GuildRoles { id: guild_id };
            pipe.smembers(key);
        }

        if C::StageInstance::WANTED {
            let key = RedisKey::GuildStageInstances { id: guild_id };
            pipe.smembers(key);
        }

        if C::Sticker::WANTED {
            let key = RedisKey::GuildStickers { id: guild_id };
            pipe.smembers(key);
        }

        if C::VoiceState::WANTED {
            let key = RedisKey::GuildVoiceStates { id: guild_id };
            pipe.smembers(key);
        }

        if pipe.cmd_iter().next().is_none() {
            if C::Guild::WANTED {
                let key = RedisKey::Guild { id: guild_id };
                pipe.del(key).ignore();

                let key = RedisKey::Guilds;
                pipe.srem(key, guild_id.get()).ignore();
            }

            return Ok(());
        }

        let mut conn = self.connection().await?;

        let mut iter = Self::query_pipe::<Vec<Vec<u64>>>(pipe, &mut conn)
            .await?
            .into_iter();

        pipe.clear();
        let mut keys_to_delete = Vec::new();

        if C::Member::WANTED {
            let user_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            if C::User::WANTED {
                for &user_id in user_ids.iter() {
                    let user_id = Id::new(user_id);

                    let key = RedisKey::UserGuilds { id: user_id };
                    pipe.srem(key, guild_id.get()).ignore();

                    let key = RedisKey::UserGuilds { id: user_id };
                    pipe.scard(key);
                }

                let scards: Vec<usize> = Self::query_pipe(pipe, &mut conn).await?;
                pipe.clear();

                let user_keys = user_ids
                    .iter()
                    .zip(scards)
                    .filter(|(_, common_guild_count)| *common_guild_count == 0)
                    .map(|(user_id, _)| RedisKey::User {
                        id: Id::new(*user_id),
                    });

                keys_to_delete.extend(user_keys);

                let key = RedisKey::Users;
                pipe.srem(key, &user_ids).ignore();
            }

            let key = RedisKey::GuildMembers { id: guild_id };
            keys_to_delete.push(key);

            let member_keys = user_ids.iter().map(|&user_id| RedisKey::Member {
                guild: guild_id,
                user: Id::new(user_id),
            });

            keys_to_delete.extend(member_keys);
        }

        if C::Channel::WANTED {
            let key = RedisKey::GuildChannels { id: guild_id };
            keys_to_delete.push(key);

            let channel_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Channels;
            pipe.srem(key, channel_ids.as_slice()).ignore();

            let channel_keys = channel_ids.into_iter().map(|channel_id| RedisKey::Channel {
                id: Id::new(channel_id),
            });

            keys_to_delete.extend(channel_keys);
        }

        if C::Emoji::WANTED {
            let key = RedisKey::GuildEmojis { id: guild_id };
            keys_to_delete.push(key);

            let emoji_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Emojis;
            pipe.srem(key, emoji_ids.as_slice()).ignore();

            let emoji_keys = emoji_ids.into_iter().map(|emoji_id| RedisKey::Emoji {
                id: Id::new(emoji_id),
            });

            keys_to_delete.extend(emoji_keys);
        }

        if C::Integration::WANTED {
            let key = RedisKey::GuildIntegrations { id: guild_id };
            keys_to_delete.push(key);

            let integration_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let integration_keys =
                integration_ids
                    .into_iter()
                    .map(|integration_id| RedisKey::Integration {
                        guild: guild_id,
                        id: Id::new(integration_id),
                    });

            keys_to_delete.extend(integration_keys);
        }

        if C::Presence::WANTED {
            let key = RedisKey::GuildPresences { id: guild_id };
            keys_to_delete.push(key);

            let user_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let presence_keys = user_ids.into_iter().map(|user_id| RedisKey::Presence {
                guild: guild_id,
                user: Id::new(user_id),
            });

            keys_to_delete.extend(presence_keys);
        }

        if C::Role::WANTED {
            let key = RedisKey::GuildRoles { id: guild_id };
            keys_to_delete.push(key);

            let role_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Roles;
            pipe.srem(key, role_ids.as_slice()).ignore();

            let role_keys = role_ids.into_iter().map(|role_id| RedisKey::Role {
                id: Id::new(role_id),
            });

            keys_to_delete.extend(role_keys);
        }

        if C::StageInstance::WANTED {
            let key = RedisKey::GuildStageInstances { id: guild_id };
            keys_to_delete.push(key);

            let stage_instance_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::StageInstances;
            pipe.srem(key, stage_instance_ids.as_slice()).ignore();

            let stage_instance_keys =
                stage_instance_ids
                    .into_iter()
                    .map(|stage_instance_id| RedisKey::StageInstance {
                        id: Id::new(stage_instance_id),
                    });

            keys_to_delete.extend(stage_instance_keys);
        }

        if C::Sticker::WANTED {
            let key = RedisKey::GuildStickers { id: guild_id };
            keys_to_delete.push(key);

            let sticker_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Stickers;
            pipe.srem(key, sticker_ids.as_slice()).ignore();

            let sticker_keys = sticker_ids.into_iter().map(|sticker_id| RedisKey::Sticker {
                id: Id::new(sticker_id),
            });

            keys_to_delete.extend(sticker_keys);
        }

        if C::VoiceState::WANTED {
            let key = RedisKey::GuildVoiceStates { id: guild_id };
            keys_to_delete.push(key);

            let user_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let voice_state_keys = user_ids.into_iter().map(|user_id| RedisKey::VoiceState {
                guild: guild_id,
                user: Id::new(user_id),
            });

            keys_to_delete.extend(voice_state_keys);
        }

        if C::Guild::WANTED {
            let key = RedisKey::Guild { id: guild_id };
            keys_to_delete.push(key);

            let key = RedisKey::Guilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        if !keys_to_delete.is_empty() {
            pipe.del(keys_to_delete).ignore();
        }

        Ok(())
    }

    async fn delete_guilds(&self, pipe: &mut Pipeline, guild_ids: &[u64]) -> CacheResult<()> {
        debug_assert!(pipe.cmd_iter().next().is_none());

        let count = C::Channel::WANTED as usize
            + C::Emoji::WANTED as usize
            + C::Integration::WANTED as usize
            + C::Member::WANTED as usize
            + C::Presence::WANTED as usize
            + C::Role::WANTED as usize
            + C::StageInstance::WANTED as usize
            + C::Sticker::WANTED as usize
            + C::VoiceState::WANTED as usize;

        if C::Channel::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildChannels {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Emoji::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildEmojis {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Integration::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildIntegrations {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Member::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildMembers {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Presence::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildPresences {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Role::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildRoles {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::StageInstance::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildStageInstances {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Sticker::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildStickers {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::VoiceState::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildVoiceStates {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if pipe.cmd_iter().next().is_none() {
            if C::Guild::WANTED {
                let guild_keys: Vec<_> = guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::Guild {
                        id: Id::new(guild_id),
                    })
                    .collect();

                pipe.del(guild_keys).ignore();

                let key = RedisKey::Guilds;
                pipe.srem(key, guild_ids).ignore();
            }

            return Ok(());
        }

        let mut conn = self.connection().await?;
        let data = Self::query_pipe::<Vec<Vec<u64>>>(pipe, &mut conn).await?;

        if data.len() != count * guild_ids.len() {
            return Err(CacheError::InvalidResponse);
        }

        let mut iter = data.into_iter();

        pipe.clear();
        let mut keys_to_delete = Vec::new();

        if C::Member::WANTED {
            let user_ids_unflattened = &iter.as_slice()[..guild_ids.len()];

            if C::User::WANTED {
                let user_ids: Vec<_> = user_ids_unflattened.iter().flatten().copied().collect();

                for (user_ids, guild_id) in user_ids_unflattened.iter().zip(guild_ids) {
                    for &user_id in user_ids {
                        let user_id = Id::new(user_id);

                        let key = RedisKey::UserGuilds { id: user_id };
                        pipe.srem(key, guild_id).ignore();

                        let key = RedisKey::UserGuilds { id: user_id };
                        pipe.scard(key);
                    }
                }

                let scards: Vec<usize> = Self::query_pipe(pipe, &mut conn).await?;
                pipe.clear();

                let key = RedisKey::Users;
                pipe.srem(key, &user_ids).ignore();

                let user_keys = user_ids
                    .iter()
                    .zip(scards)
                    .filter(|(_, common_guild_count)| *common_guild_count == 0)
                    .map(|(user_id, _)| RedisKey::User {
                        id: Id::new(*user_id),
                    });

                keys_to_delete.extend(user_keys);
            }

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildMembers {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);

            let member_keys =
                user_ids_unflattened
                    .iter()
                    .zip(guild_ids)
                    .flat_map(|(user_ids, guild_id)| {
                        user_ids.iter().map(|&user_id| RedisKey::Member {
                            guild: Id::new(*guild_id),
                            user: Id::new(user_id),
                        })
                    });

            keys_to_delete.extend(member_keys);

            iter.by_ref().take(guild_ids.len()).for_each(|_| ());
        }

        if C::Channel::WANTED {
            let channel_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Channels;
            pipe.srem(key, channel_ids.as_slice()).ignore();

            let channel_keys = channel_ids.into_iter().map(|emoji_id| RedisKey::Channel {
                id: Id::new(emoji_id),
            });

            keys_to_delete.extend(channel_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildChannels {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Emoji::WANTED {
            let emoji_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Emojis;
            pipe.srem(key, emoji_ids.as_slice()).ignore();

            let emoji_keys = emoji_ids.into_iter().map(|emoji_id| RedisKey::Emoji {
                id: Id::new(emoji_id),
            });

            keys_to_delete.extend(emoji_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildEmojis {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Integration::WANTED {
            let integration_keys = iter.by_ref().take(guild_ids.len()).zip(guild_ids).flat_map(
                |(integration_ids, guild_id)| {
                    integration_ids
                        .into_iter()
                        .map(|integration_id| RedisKey::Integration {
                            guild: Id::new(*guild_id),
                            id: Id::new(integration_id),
                        })
                },
            );

            keys_to_delete.extend(integration_keys);

            let guild_keys =
                guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::GuildIntegrations {
                        id: Id::new(guild_id),
                    });

            keys_to_delete.extend(guild_keys);
        }

        if C::Presence::WANTED {
            let presence_keys = iter.by_ref().take(guild_ids.len()).zip(guild_ids).flat_map(
                |(user_ids, guild_id)| {
                    user_ids.into_iter().map(|user_id| RedisKey::Presence {
                        guild: Id::new(*guild_id),
                        user: Id::new(user_id),
                    })
                },
            );

            keys_to_delete.extend(presence_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildPresences {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Role::WANTED {
            let role_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Roles;
            pipe.srem(key, role_ids.as_slice()).ignore();

            let role_keys = role_ids.into_iter().map(|role_id| RedisKey::Role {
                id: Id::new(role_id),
            });

            keys_to_delete.extend(role_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildRoles {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::StageInstance::WANTED {
            let stage_instance_ids: Vec<_> =
                iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::StageInstances;
            pipe.srem(key, stage_instance_ids.as_slice()).ignore();

            let stage_instance_keys =
                stage_instance_ids
                    .into_iter()
                    .map(|stage_instance_id| RedisKey::StageInstance {
                        id: Id::new(stage_instance_id),
                    });

            keys_to_delete.extend(stage_instance_keys);

            let guild_keys =
                guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::GuildStageInstances {
                        id: Id::new(guild_id),
                    });

            keys_to_delete.extend(guild_keys);
        }

        if C::Sticker::WANTED {
            let sticker_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Stickers;
            pipe.srem(key, sticker_ids.as_slice()).ignore();

            let sticker_keys = sticker_ids.into_iter().map(|sticker_id| RedisKey::Sticker {
                id: Id::new(sticker_id),
            });

            keys_to_delete.extend(sticker_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildStickers {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::VoiceState::WANTED {
            let voice_state_keys = iter.by_ref().take(guild_ids.len()).zip(guild_ids).flat_map(
                |(user_ids, guild_id)| {
                    user_ids.into_iter().map(|user_id| RedisKey::VoiceState {
                        guild: Id::new(*guild_id),
                        user: Id::new(user_id),
                    })
                },
            );

            keys_to_delete.extend(voice_state_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildVoiceStates {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Guild::WANTED {
            let guild_keys = guild_ids.iter().copied().map(|guild_id| RedisKey::Guild {
                id: Id::new(guild_id),
            });

            keys_to_delete.extend(guild_keys);

            let key = RedisKey::Guilds;
            pipe.srem(key, guild_ids).ignore();
        }

        if !keys_to_delete.is_empty() {
            pipe.del(keys_to_delete).ignore();
        }

        Ok(())
    }

    fn delete_integration(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        integration_id: Id<IntegrationMarker>,
    ) {
        if !C::Integration::WANTED {
            return;
        }

        let key = RedisKey::Integration {
            guild: guild_id,
            id: integration_id,
        };
        pipe.del(key).ignore();

        let key = RedisKey::GuildIntegrations { id: guild_id };
        pipe.srem(key, integration_id.get()).ignore();
    }

    async fn delete_member(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<()> {
        if !C::Member::WANTED {
            return Ok(());
        }

        if C::User::WANTED {
            debug_assert!(pipe.cmd_iter().next().is_none());

            let key = RedisKey::UserGuilds { id: user_id };
            pipe.srem(key, guild_id.get()).ignore();

            let key = RedisKey::UserGuilds { id: user_id };
            pipe.scard(key);

            let mut conn = self.connection().await?;
            let common_guild_count: usize = Self::query_pipe(pipe, &mut conn).await?;
            pipe.clear();

            if common_guild_count == 0 {
                let key = RedisKey::User { id: user_id };
                pipe.del(key).ignore();

                let key = RedisKey::Users;
                pipe.srem(key, user_id.get()).ignore();
            }
        }

        let key = RedisKey::Member {
            guild: guild_id,
            user: user_id,
        };
        pipe.del(key).ignore();

        let key = RedisKey::GuildMembers { id: guild_id };
        pipe.srem(key, user_id.get()).ignore();

        Ok(())
    }

    fn delete_message(&self, pipe: &mut Pipeline, msg_id: Id<MessageMarker>) {
        if !C::Message::WANTED {
            return;
        }

        let key = RedisKey::Message { id: msg_id };
        pipe.del(key).ignore();
    }

    fn delete_messages(&self, pipe: &mut Pipeline, msg_ids: &[Id<MessageMarker>]) {
        if !C::Message::WANTED || msg_ids.is_empty() {
            return;
        }

        let keys: Vec<_> = msg_ids
            .iter()
            .copied()
            .map(|id| RedisKey::Message { id })
            .collect();

        pipe.del(keys).ignore();
    }

    fn delete_role(&self, pipe: &mut Pipeline, guild_id: Id<GuildMarker>, role_id: Id<RoleMarker>) {
        if !C::Role::WANTED {
            return;
        }

        let key = RedisKey::Role { id: role_id };
        pipe.del(key).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.srem(key, role_id.get()).ignore();

        let key = RedisKey::Roles;
        pipe.srem(key, role_id.get()).ignore();
    }

    fn delete_stage_instance(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        stage_instance_id: Id<StageMarker>,
    ) {
        if !C::StageInstance::WANTED {
            return;
        }

        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        pipe.del(key).ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.srem(key, stage_instance_id.get()).ignore();

        let key = RedisKey::StageInstances;
        pipe.srem(key, stage_instance_id.get()).ignore();
    }

    fn delete_voice_state(
        &self,
        pipe: &mut Pipeline,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) {
        if !C::VoiceState::WANTED {
            return;
        }

        let key = RedisKey::VoiceState {
            guild: guild_id,
            user: user_id,
        };
        pipe.del(key).ignore();

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.srem(key, user_id.get()).ignore();
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
