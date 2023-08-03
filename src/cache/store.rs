use twilight_model::channel::message::Sticker;
use twilight_model::channel::{Channel, Message, StageInstance};
use twilight_model::gateway::payload::incoming::invite_create::PartialUser;
use twilight_model::gateway::payload::incoming::{MemberUpdate, MessageUpdate};
use twilight_model::gateway::presence::{Presence, UserOrId};
use twilight_model::guild::{
    Emoji, Guild, GuildIntegration, Member, PartialGuild, PartialMember, Role, UnavailableGuild,
};
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, User};
use twilight_model::voice::VoiceState;

use crate::CacheError;
use crate::{
    config::{
        CacheConfig, Cacheable, ICachedChannel, ICachedCurrentUser, ICachedEmoji, ICachedGuild,
        ICachedIntegration, ICachedMember, ICachedMessage, ICachedPresence, ICachedRole,
        ICachedStageInstance, ICachedSticker, ICachedUser, ICachedVoiceState,
    },
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

use super::pipe::Pipe;

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
    pub(crate) fn store_channel(
        &self,
        pipe: &mut Pipe<'_, C>,
        channel: &Channel,
    ) -> CacheResult<()> {
        if C::Channel::WANTED {
            let guild_id = channel.guild_id;
            let channel_id = channel.id;
            let key = RedisKey::Channel { id: channel_id };
            let channel = C::Channel::from_channel(channel);

            let bytes = channel
                .serialize()
                .map_err(|e| SerializeError::Channel(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Channel::expire_seconds())
                .ignore();

            if let Some(guild_id) = guild_id {
                let key = RedisKey::GuildChannels { id: guild_id };
                pipe.sadd(key, channel_id.get()).ignore();
            }

            let key = RedisKey::Channels;
            pipe.sadd(key, channel_id.get()).ignore();
        }

        if let Some(ref member) = channel.member {
            if let (Some(guild_id), Some(member)) = (channel.guild_id, &member.member) {
                self.store_member(pipe, guild_id, member)?;
            }

            if let Some(ref presence) = member.presence {
                self.store_presence(pipe, presence)?;
            }
        }

        if let Some(ref users) = channel.recipients {
            self.store_users(pipe, users)?;
        }

        Ok(())
    }

    pub(crate) fn store_channels(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        channels: &[Channel],
    ) -> CacheResult<()> {
        if C::Channel::WANTED {
            let mut serializer = ChannelSerializer::<C>::default();

            let (channels, channel_ids) = channels
                .iter()
                .map(|channel| {
                    let id = channel.id;
                    let key = RedisKey::Channel { id };
                    let channel = C::Channel::from_channel(channel);

                    let bytes = channel
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError::Channel(Box::new(e)))?;

                    Ok(((key, BytesArg(bytes)), id.get()))
                })
                .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
                .unzip();

            if !channels.is_empty() {
                pipe.mset(&channels, C::Channel::expire_seconds()).ignore();

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

        self.store_users(pipe, users)?;

        Ok(())
    }

    pub(crate) fn store_current_user(
        &self,
        pipe: &mut Pipe<'_, C>,
        current_user: &CurrentUser,
    ) -> CacheResult<()> {
        if !C::CurrentUser::WANTED {
            return Ok(());
        }

        let key = RedisKey::CurrentUser;
        let current_user = C::CurrentUser::from_current_user(current_user);

        let bytes = current_user
            .serialize()
            .map_err(|e| SerializeError::CurrentUser(Box::new(e)))?;

        pipe.set(key, bytes.as_ref(), C::CurrentUser::expire_seconds())
            .ignore();

        Ok(())
    }

    pub(crate) fn store_emojis(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        emojis: &[Emoji],
    ) -> CacheResult<()> {
        if !C::Emoji::WANTED {
            return Ok(());
        }

        let mut serializer = EmojiSerializer::<C>::default();

        let (emojis, emoji_ids) = emojis
            .iter()
            .map(|emoji| {
                let id = emoji.id;
                let key = RedisKey::Emoji { id };
                let emoji = C::Emoji::from_emoji(emoji);

                let bytes = emoji
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::Emoji(Box::new(e)))?;

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if emojis.is_empty() {
            return Ok(());
        }

        pipe.mset(&emojis, C::Emoji::expire_seconds()).ignore();

        let key = RedisKey::GuildEmojis { id: guild_id };
        pipe.sadd(key, emoji_ids.as_slice()).ignore();

        let key = RedisKey::Emojis;
        pipe.sadd(key, emoji_ids).ignore();

        Ok(())
    }

    pub(crate) fn store_guild(&self, pipe: &mut Pipe<'_, C>, guild: &Guild) -> CacheResult<()> {
        if C::Guild::WANTED {
            let guild_id = guild.id;
            let key = RedisKey::Guild { id: guild_id };
            let guild = C::Guild::from_guild(guild);

            let bytes = guild
                .serialize()
                .map_err(|e| SerializeError::Guild(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Guild::expire_seconds())
                .ignore();

            let key = RedisKey::Guilds;
            pipe.sadd(key, guild_id.get()).ignore();

            let key = RedisKey::UnavailableGuilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        self.store_channels(pipe, guild.id, &guild.channels)?;
        self.store_emojis(pipe, guild.id, &guild.emojis)?;
        self.store_members(pipe, guild.id, &guild.members)?;
        self.store_presences(pipe, guild.id, &guild.presences)?;
        self.store_roles(pipe, guild.id, &guild.roles)?;
        self.store_stickers(pipe, guild.id, &guild.stickers)?;
        self.store_channels(pipe, guild.id, &guild.threads)?;
        self.store_stage_instances(pipe, guild.id, &guild.stage_instances)?;
        self.store_voice_states(pipe, guild.id, &guild.voice_states)?;

        Ok(())
    }

    pub(crate) fn store_integration(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        integration: &GuildIntegration,
    ) -> CacheResult<()> {
        if C::Integration::WANTED {
            let integration_id = integration.id;
            let key = RedisKey::Integration {
                guild: guild_id,
                id: integration_id,
            };
            let integration = C::Integration::from_integration(integration);

            let bytes = integration
                .serialize()
                .map_err(|e| SerializeError::Integration(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Integration::expire_seconds())
                .ignore();

            let key = RedisKey::GuildIntegrations { id: guild_id };
            pipe.sadd(key, integration_id.get()).ignore();
        }

        if let Some(ref user) = integration.user {
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    pub(crate) fn store_member(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        member: &Member,
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let user_id = member.user.id;
            let key = RedisKey::Member {
                guild: guild_id,
                user: user_id,
            };
            let member = C::Member::from_member(guild_id, member);

            let bytes = member
                .serialize()
                .map_err(|e| SerializeError::Member(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Member::expire_seconds())
                .ignore();

            let key = RedisKey::GuildMembers { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();

            if C::User::WANTED {
                let key = RedisKey::UserGuilds { id: user_id };
                pipe.sadd(key, guild_id.get()).ignore();
            }
        }

        self.store_user(pipe, &member.user)?;

        Ok(())
    }

    pub(crate) fn store_member_update(
        &self,
        pipe: &mut Pipe<'_, C>,
        update: &MemberUpdate,
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let user_id = update.user.id;
            let key = RedisKey::Member {
                guild: update.guild_id,
                user: user_id,
            };
            if let Some(member) = C::Member::from_member_update(update) {
                let bytes = member
                    .serialize()
                    .map_err(|e| SerializeError::Member(Box::new(e)))?;

                pipe.set(key, bytes.as_ref(), C::Member::expire_seconds())
                    .ignore();
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

        self.store_user(pipe, &update.user)?;

        Ok(())
    }

    pub(crate) fn store_members(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        members: &[Member],
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let mut serializer = MemberSerializer::<C>::default();

            let (member_tuples, user_ids) = members
                .iter()
                .map(|member| {
                    let user_id = member.user.id;
                    let key = RedisKey::Member {
                        guild: guild_id,
                        user: user_id,
                    };
                    let member = C::Member::from_member(guild_id, member);

                    let bytes = member
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError::Member(Box::new(e)))?;

                    Ok(((key, BytesArg(bytes)), user_id.get()))
                })
                .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
                .unzip();

            if !member_tuples.is_empty() {
                pipe.mset(&member_tuples, C::Member::expire_seconds())
                    .ignore();

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
        self.store_users(pipe, users)?;

        Ok(())
    }

    pub(crate) fn store_message(&self, pipe: &mut Pipe<'_, C>, msg: &Message) -> CacheResult<()> {
        if C::Message::WANTED {
            let key = RedisKey::Message { id: msg.id };
            let msg = C::Message::from_message(msg);

            let bytes = msg
                .serialize()
                .map_err(|e| SerializeError::Message(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Message::expire_seconds())
                .ignore();
        }

        self.store_user(pipe, &msg.author)?;

        if let (Some(guild_id), Some(member)) = (msg.guild_id, &msg.member) {
            self.store_partial_member(pipe, guild_id, member)?;
        }

        if let Some(ref channel) = msg.thread {
            self.store_channel(pipe, channel)?;
        }

        Ok(())
    }

    pub(crate) fn store_message_update(
        &self,
        pipe: &mut Pipe<'_, C>,
        update: &MessageUpdate,
    ) -> CacheResult<()> {
        if C::Message::WANTED {
            if let Some(msg) = C::Message::from_message_update(update) {
                let key = RedisKey::Message { id: update.id };

                let bytes = msg
                    .serialize()
                    .map_err(|e| SerializeError::Message(Box::new(e)))?;

                pipe.set(key, bytes.as_ref(), C::Message::expire_seconds())
                    .ignore();
            }
        }

        if let Some(ref user) = update.author {
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    pub(crate) fn store_partial_guild(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild: &PartialGuild,
    ) -> CacheResult<()> {
        if C::Guild::WANTED {
            let guild_id = guild.id;

            if let Some(guild) = C::Guild::from_partial_guild(guild) {
                let key = RedisKey::Guild { id: guild_id };

                let bytes = guild
                    .serialize()
                    .map_err(|e| SerializeError::Guild(Box::new(e)))?;

                pipe.set(key, bytes.as_ref(), C::Guild::expire_seconds())
                    .ignore();
            }

            let key = RedisKey::Guilds;
            pipe.sadd(key, guild_id.get()).ignore();

            let key = RedisKey::UnavailableGuilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        self.store_emojis(pipe, guild.id, &guild.emojis)?;
        self.store_roles(pipe, guild.id, &guild.roles)?;

        Ok(())
    }

    pub(crate) fn store_partial_member(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        member: &PartialMember,
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            if let Some(ref user) = member.user {
                if let Some(member) = C::Member::from_partial_member(guild_id, member) {
                    let key = RedisKey::Member {
                        guild: guild_id,
                        user: user.id,
                    };

                    let bytes = member
                        .serialize()
                        .map_err(|e| SerializeError::Member(Box::new(e)))?;

                    pipe.set(key, bytes.as_ref(), C::Member::expire_seconds())
                        .ignore();
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
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    pub(crate) fn store_partial_user(
        &self,
        pipe: &mut Pipe<'_, C>,
        user: &PartialUser,
    ) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        let id = user.id;
        let key = RedisKey::User { id };

        if let Some(user) = C::User::from_partial_user(user) {
            let bytes = user
                .serialize()
                .map_err(|e| SerializeError::User(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::User::expire_seconds())
                .ignore();
        }

        let key = RedisKey::Users;
        pipe.sadd(key, id.get()).ignore();

        Ok(())
    }

    pub(crate) fn store_presence(
        &self,
        pipe: &mut Pipe<'_, C>,
        presence: &Presence,
    ) -> CacheResult<()> {
        if C::Presence::WANTED {
            let guild_id = presence.guild_id;
            let user_id = presence.user.id();
            let key = RedisKey::Presence {
                guild: guild_id,
                user: user_id,
            };
            let presence = C::Presence::from_presence(presence);

            let bytes = presence
                .serialize()
                .map_err(|e| SerializeError::Presence(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Presence::expire_seconds())
                .ignore();

            let key = RedisKey::GuildPresences { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if let UserOrId::User(ref user) = presence.user {
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    pub(crate) fn store_presences(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        presences: &[Presence],
    ) -> CacheResult<()> {
        if C::Presence::WANTED {
            let mut serializer = PresenceSerializer::<C>::default();

            let (presences, user_ids) = presences
                .iter()
                .map(|presence| {
                    let guild_id = presence.guild_id;
                    let user_id = presence.user.id();
                    let key = RedisKey::Presence {
                        guild: guild_id,
                        user: user_id,
                    };
                    let presence = C::Presence::from_presence(presence);

                    let bytes = presence
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError::Presence(Box::new(e)))?;

                    Ok(((key, BytesArg(bytes)), user_id.get()))
                })
                .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
                .unzip();

            if !presences.is_empty() {
                pipe.mset(&presences, C::Presence::expire_seconds())
                    .ignore();

                let key = RedisKey::GuildPresences { id: guild_id };
                pipe.sadd(key, user_ids.as_slice()).ignore();
            }
        }

        let users = presences.iter().filter_map(|presence| match presence.user {
            UserOrId::User(ref user) => Some(user),
            UserOrId::UserId { .. } => None,
        });

        self.store_users(pipe, users)?;

        Ok(())
    }

    pub(crate) fn store_role(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        role: &Role,
    ) -> CacheResult<()> {
        if !C::Role::WANTED {
            return Ok(());
        }

        let id = role.id;
        let key = RedisKey::Role { id };
        let role = C::Role::from_role(role);

        let bytes = role
            .serialize()
            .map_err(|e| SerializeError::Role(Box::new(e)))?;

        pipe.set(key, bytes.as_ref(), C::Role::expire_seconds())
            .ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, id.get()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, id.get()).ignore();

        Ok(())
    }

    pub(crate) fn store_roles<'a, I>(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        roles: I,
    ) -> CacheResult<()>
    where
        I: IntoIterator<Item = &'a Role>,
    {
        if !C::Role::WANTED {
            return Ok(());
        }

        let mut serializer = RoleSerializer::<C>::default();

        let (roles, role_ids) = roles
            .into_iter()
            .map(|role| {
                let id = role.id;
                let key = RedisKey::Role { id };
                let role = C::Role::from_role(role);

                let bytes = role
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::Role(Box::new(e)))?;

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if roles.is_empty() {
            return Ok(());
        }

        pipe.mset(&roles, C::Role::expire_seconds()).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, role_ids.as_slice()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, role_ids).ignore();

        Ok(())
    }

    pub(crate) fn store_stage_instance(
        &self,
        pipe: &mut Pipe<'_, C>,
        stage_instance: &StageInstance,
    ) -> CacheResult<()> {
        if !C::StageInstance::WANTED {
            return Ok(());
        }

        let stage_instance_id = stage_instance.id;
        let guild_id = stage_instance.guild_id;
        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        let stage_instance = C::StageInstance::from_stage_instance(stage_instance);

        let bytes = stage_instance
            .serialize()
            .map_err(|e| SerializeError::StageInstance(Box::new(e)))?;

        pipe.set(key, bytes.as_ref(), C::StageInstance::expire_seconds())
            .ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_id.get()).ignore();

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_id.get()).ignore();

        Ok(())
    }

    pub(crate) fn store_stage_instances(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        stage_instances: &[StageInstance],
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
                let bytes = stage_instance
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::StageInstance(Box::new(e)))?;

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if stage_instances.is_empty() {
            return Ok(());
        }

        pipe.mset(&stage_instances, C::StageInstance::expire_seconds())
            .ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_ids.as_slice()).ignore();

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_ids).ignore();

        Ok(())
    }

    pub(crate) fn store_stickers(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        stickers: &[Sticker],
    ) -> CacheResult<()> {
        if !C::Sticker::WANTED {
            return Ok(());
        }

        let mut serializer = StickerSerializer::<C>::default();

        let (stickers, sticker_ids) = stickers
            .iter()
            .map(|sticker| {
                let id = sticker.id;
                let key = RedisKey::Sticker { id };
                let sticker = C::Sticker::from_sticker(sticker);

                let bytes = sticker
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::Sticker(Box::new(e)))?;

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if stickers.is_empty() {
            return Ok(());
        }

        pipe.mset(&stickers, C::Sticker::expire_seconds()).ignore();

        let key = RedisKey::GuildStickers { id: guild_id };
        pipe.sadd(key, sticker_ids.as_slice()).ignore();

        let key = RedisKey::Stickers;
        pipe.sadd(key, sticker_ids).ignore();

        Ok(())
    }

    pub(crate) async fn store_unavailable_guild(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        self.delete_guild(pipe, guild_id).await?;

        let key = RedisKey::UnavailableGuilds;
        pipe.sadd(key, guild_id.get()).ignore();

        Ok(())
    }

    pub(crate) async fn store_unavailable_guilds(
        &self,
        pipe: &mut Pipe<'_, C>,
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

    pub(crate) fn store_user(&self, pipe: &mut Pipe<'_, C>, user: &User) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        let id = user.id;
        let key = RedisKey::User { id };
        let user = C::User::from_user(user);

        let bytes = user
            .serialize()
            .map_err(|e| SerializeError::User(Box::new(e)))?;

        pipe.set(key, bytes.as_ref(), C::User::expire_seconds())
            .ignore();

        let key = RedisKey::Users;
        pipe.sadd(key, id.get()).ignore();

        Ok(())
    }

    pub(crate) fn store_users<'a, I>(&self, pipe: &mut Pipe<'_, C>, users: I) -> CacheResult<()>
    where
        I: IntoIterator<Item = &'a User>,
    {
        if !C::User::WANTED {
            return Ok(());
        }

        let mut serializer = UserSerializer::<C>::default();

        let (users, user_ids) = users
            .into_iter()
            .map(|user| {
                let id = user.id;
                let key = RedisKey::User { id };
                let user = C::User::from_user(user);

                let bytes = user
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::User(Box::new(e)))?;

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if users.is_empty() {
            return Ok(());
        }

        pipe.mset(&users, C::User::expire_seconds()).ignore();

        let key = RedisKey::Users;
        pipe.sadd(key, user_ids).ignore();

        Ok(())
    }

    pub(crate) fn store_voice_state(
        &self,
        pipe: &mut Pipe<'_, C>,
        channel_id: Id<ChannelMarker>,
        voice_state: &VoiceState,
    ) -> CacheResult<()> {
        let Some(guild_id) = voice_state.guild_id else {
            return Ok(());
        };

        if C::VoiceState::WANTED {
            let user_id = voice_state.user_id;
            let key = RedisKey::VoiceState {
                guild: guild_id,
                user: user_id,
            };
            let voice_state = C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);

            let bytes = voice_state
                .serialize()
                .map_err(|e| SerializeError::VoiceState(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::VoiceState::expire_seconds())
                .ignore();

            let key = RedisKey::GuildVoiceStates { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if let Some(ref member) = voice_state.member {
            self.store_member(pipe, guild_id, member)?;
        }

        Ok(())
    }

    pub(crate) fn store_voice_states(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        voice_states: &[VoiceState],
    ) -> CacheResult<()> {
        if !C::VoiceState::WANTED {
            return Ok(());
        }

        let mut serializer = VoiceStateSerializer::<C>::default();

        let (voice_states, user_ids) = voice_states
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

                let res = voice_state
                    .serialize_with(&mut serializer)
                    .map(|bytes| ((key, BytesArg(bytes)), user_id.get()))
                    .map_err(|e| {
                        CacheError::Serialization(SerializeError::VoiceState(Box::new(e)))
                    });

                Some(res)
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if voice_states.is_empty() {
            return Ok(());
        }

        pipe.mset(&voice_states, C::VoiceState::expire_seconds())
            .ignore();

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.sadd(key, user_ids.as_slice()).ignore();

        Ok(())
    }
}
