use std::collections::HashSet;

use twilight_model::id::{
    marker::{
        ChannelMarker, EmojiMarker, GuildMarker, IntegrationMarker, MessageMarker, RoleMarker,
        StageMarker, StickerMarker, UserMarker,
    },
    Id,
};

use crate::{
    config::CacheConfig,
    key::RedisKey,
    redis::{Cmd, FromRedisValue},
    CacheError, CacheResult, CachedArchive, RedisCache,
};

use super::Connection;

impl<C: CacheConfig> RedisCache<C> {
    pub async fn channel(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Channel<'static>>>> {
        self.get_single(channel_id).await
    }

    pub async fn channel_ids(&self) -> CacheResult<HashSet<Id<ChannelMarker>>> {
        self.get_ids(RedisKey::Channels).await
    }

    pub async fn channel_msg_ids(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<HashSet<Id<MessageMarker>>> {
        let key = RedisKey::ChannelMessages {
            channel: channel_id,
        };

        self.get_ids(key).await
    }

    pub async fn common_guild_ids(
        &self,
        user_id: Id<UserMarker>,
    ) -> CacheResult<HashSet<Id<GuildMarker>>> {
        self.get_ids(RedisKey::UserGuilds { id: user_id }).await
    }

    pub async fn current_user(
        &self,
    ) -> CacheResult<Option<CachedArchive<C::CurrentUser<'static>>>> {
        self.get_single(RedisKey::CurrentUser).await
    }

    pub async fn emoji(
        &self,
        emoji_id: Id<EmojiMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Emoji<'static>>>> {
        self.get_single(emoji_id).await
    }

    pub async fn guild(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Guild<'static>>>> {
        self.get_single(guild_id).await
    }

    pub async fn guild_channel_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<ChannelMarker>>> {
        self.get_ids(RedisKey::GuildChannels { id: guild_id }).await
    }

    pub async fn guild_emoji_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<EmojiMarker>>> {
        self.get_ids(RedisKey::GuildEmojis { id: guild_id }).await
    }

    pub async fn guild_integration_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<IntegrationMarker>>> {
        self.get_ids(RedisKey::GuildIntegrations { id: guild_id })
            .await
    }

    pub async fn guild_member_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<UserMarker>>> {
        self.get_ids(RedisKey::GuildMembers { id: guild_id }).await
    }

    pub async fn guild_presence_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<UserMarker>>> {
        self.get_ids(RedisKey::GuildPresences { id: guild_id })
            .await
    }

    pub async fn guild_role_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<RoleMarker>>> {
        self.get_ids(RedisKey::GuildRoles { id: guild_id }).await
    }

    pub async fn guild_stage_instance_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<StageMarker>>> {
        self.get_ids(RedisKey::GuildStageInstances { id: guild_id })
            .await
    }

    pub async fn guild_sticker_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<StickerMarker>>> {
        self.get_ids(RedisKey::GuildStickers { id: guild_id }).await
    }

    pub async fn guild_voice_state_ids(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<HashSet<Id<UserMarker>>> {
        self.get_ids(RedisKey::GuildVoiceStates { id: guild_id })
            .await
    }

    pub async fn guild_ids(&self) -> CacheResult<HashSet<Id<GuildMarker>>> {
        self.get_ids(RedisKey::Guilds).await
    }

    pub async fn integration(
        &self,
        guild_id: Id<GuildMarker>,
        integration_id: Id<IntegrationMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Integration<'static>>>> {
        let key = RedisKey::Integration {
            guild: guild_id,
            id: integration_id,
        };

        self.get_single(key).await
    }

    pub async fn member(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Member<'static>>>> {
        let key = RedisKey::Member {
            guild: guild_id,
            user: user_id,
        };

        self.get_single(key).await
    }

    pub async fn message(
        &self,
        msg_id: Id<MessageMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Message<'static>>>> {
        self.get_single(msg_id).await
    }

    pub async fn message_ids(&self) -> CacheResult<HashSet<Id<MessageMarker>>> {
        self.get_ids(RedisKey::Messages).await
    }

    pub async fn presence(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Presence<'static>>>> {
        let key = RedisKey::Presence {
            guild: guild_id,
            user: user_id,
        };

        self.get_single(key).await
    }

    pub async fn role(
        &self,
        role_id: Id<RoleMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Role<'static>>>> {
        self.get_single(role_id).await
    }

    pub async fn role_ids(&self) -> CacheResult<HashSet<Id<RoleMarker>>> {
        self.get_ids(RedisKey::Roles).await
    }

    pub async fn stage_instance(
        &self,
        stage_instance_id: Id<StageMarker>,
    ) -> CacheResult<Option<CachedArchive<C::StageInstance<'static>>>> {
        self.get_single(stage_instance_id).await
    }

    pub async fn sticker(
        &self,
        sticker_id: Id<StickerMarker>,
    ) -> CacheResult<Option<CachedArchive<C::Sticker<'static>>>> {
        self.get_single(sticker_id).await
    }

    pub async fn unavailable_guild_ids(&self) -> CacheResult<HashSet<Id<GuildMarker>>> {
        self.get_ids(RedisKey::UnavailableGuilds).await
    }

    pub async fn unavailable_guilds_count(&self) -> CacheResult<usize> {
        let mut conn = self.connection().await?;
        let key = RedisKey::UnavailableGuilds;
        let count = Cmd::scard(key).query_async(&mut conn).await?;

        Ok(count)
    }

    pub async fn user(
        &self,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedArchive<C::User<'static>>>> {
        self.get_single(user_id).await
    }

    pub async fn user_ids(&self) -> CacheResult<HashSet<Id<UserMarker>>> {
        self.get_ids(RedisKey::Users).await
    }

    pub async fn voice_state(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedArchive<C::VoiceState<'static>>>> {
        let key = RedisKey::VoiceState {
            guild: guild_id,
            user: user_id,
        };

        self.get_single(key).await
    }
}

impl<C> RedisCache<C> {
    #[cfg(feature = "validation")]
    async fn get_single<K, V>(&self, key: K) -> CacheResult<Option<CachedArchive<V>>>
    where
        RedisKey: From<K>,
        V: crate::config::Cacheable,
    {
        let mut conn = self.connection().await?;
        let bytes: Vec<u8> = Cmd::get(RedisKey::from(key)).query_async(&mut conn).await?;

        if bytes.is_empty() {
            return Ok(None);
        }

        CachedArchive::new(bytes.into_boxed_slice()).map(Some)
    }

    #[cfg(not(feature = "validation"))]
    async fn get_single<K, V>(&self, key: K) -> CacheResult<Option<CachedArchive<V>>>
    where
        RedisKey: From<K>,
    {
        let mut conn = self.connection().await?;
        let bytes: Vec<u8> = Cmd::get(RedisKey::from(key)).query_async(&mut conn).await?;

        if bytes.is_empty() {
            return Ok(None);
        }

        Ok(Some(CachedArchive::new_unchecked(bytes.into_boxed_slice())))
    }

    async fn get_ids<T>(&self, key: RedisKey) -> CacheResult<HashSet<Id<T>>> {
        let mut conn = self.connection().await?;

        Self::get_ids_static(key, &mut conn).await.map(convert_ids)
    }

    pub(crate) async fn get_ids_static<T>(
        key: RedisKey,
        conn: &mut Connection<'_>,
    ) -> CacheResult<T>
    where
        T: FromRedisValue,
    {
        Cmd::smembers(key)
            .query_async(conn)
            .await
            .map_err(CacheError::Redis)
    }
}

fn convert_ids<T>(ids: HashSet<u64>) -> HashSet<Id<T>> {
    // SAFETY: Id<T> is a transparent wrapper around NonZeroU64
    unsafe { std::mem::transmute(ids) }
}
