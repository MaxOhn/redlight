use std::collections::HashSet;

use twilight_model::id::{
    marker::{
        ChannelMarker, EmojiMarker, GuildMarker, IntegrationMarker, MessageMarker, RoleMarker,
        StageMarker, StickerMarker, UserMarker,
    },
    Id,
};

use crate::{
    config::{CacheConfig, Cacheable},
    key::RedisKey,
    redis::AsyncCommands,
    CacheResult, CachedValue, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    pub async fn channel(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<Option<CachedValue<C::Channel<'static>>>> {
        self.get_single(channel_id).await
    }

    pub async fn channel_ids(&self) -> CacheResult<HashSet<Id<ChannelMarker>>> {
        self.get_ids(RedisKey::Channels).await
    }

    pub async fn common_guild_ids(
        &self,
        user_id: Id<UserMarker>,
    ) -> CacheResult<HashSet<Id<GuildMarker>>> {
        self.get_ids(RedisKey::UserGuilds { id: user_id }).await
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
    ) -> CacheResult<Option<CachedValue<C::Integration<'static>>>> {
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

    pub async fn role_ids(&self) -> CacheResult<HashSet<Id<RoleMarker>>> {
        self.get_ids(RedisKey::Roles).await
    }

    pub async fn stage_instance(
        &self,
        stage_instance_id: Id<StageMarker>,
    ) -> CacheResult<Option<CachedValue<C::StageInstance<'static>>>> {
        self.get_single(stage_instance_id).await
    }

    pub async fn sticker(
        &self,
        sticker_id: Id<StickerMarker>,
    ) -> CacheResult<Option<CachedValue<C::Sticker<'static>>>> {
        self.get_single(sticker_id).await
    }

    pub async fn unavailable_guild_ids(&self) -> CacheResult<HashSet<Id<GuildMarker>>> {
        self.get_ids(RedisKey::UnavailableGuilds).await
    }

    pub async fn unavailable_guilds_count(&self) -> CacheResult<usize> {
        let mut conn = self.connection().await?;
        let key = RedisKey::UnavailableGuilds;
        let count = conn.scard(key).await?;

        Ok(count)
    }

    pub async fn user(
        &self,
        user_id: Id<UserMarker>,
    ) -> CacheResult<Option<CachedValue<C::User<'static>>>> {
        self.get_single(user_id).await
    }

    pub async fn user_ids(&self) -> CacheResult<HashSet<Id<UserMarker>>> {
        self.get_ids(RedisKey::Users).await
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

    async fn get_ids<T>(&self, key: RedisKey) -> CacheResult<HashSet<Id<T>>> {
        let mut conn = self.connection().await?;
        let ids = conn.smembers(key).await?;

        Ok(convert_ids(ids))
    }
}

fn convert_ids<T>(ids: HashSet<u64>) -> HashSet<Id<T>> {
    // SAFETY: Id<T> is a transparent wrapper around NonZeroU64
    unsafe { std::mem::transmute(ids) }
}
