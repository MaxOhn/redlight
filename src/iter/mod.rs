mod cache_iter;

use rkyv::Archived;
use twilight_model::id::{
    marker::{
        ChannelMarker, EmojiMarker, GuildMarker, IntegrationMarker, MessageMarker, RoleMarker,
        StageMarker, StickerMarker, UserMarker,
    },
    Id,
};

pub use self::cache_iter::{CacheIter, EntryResult, OptionalCacheIter};
use crate::{
    config::{CacheConfig, CheckedArchived},
    error::CacheError,
    key::RedisKey,
    redis::{Cmd, Connection},
    util::convert_ids_vec,
    CacheResult, RedisCache,
};

/// Base type to create iterators for cached entries.
///
/// The iteration order of all iterators is arbitrary, except for
/// [`RedisCacheIter::channel_messages`] whose order is the message timestamp
/// i.e. from most recent to oldest.
pub struct RedisCacheIter<'c, C> {
    cache: &'c RedisCache<C>,
}

impl<'c, C> RedisCacheIter<'c, C> {
    pub(crate) const fn new(cache: &'c RedisCache<C>) -> Self {
        Self { cache }
    }

    /// Reference to the underlying cache.
    pub const fn cache_ref(&self) -> &RedisCache<C> {
        self.cache
    }
}

impl<C: CacheConfig> RedisCacheIter<'_, C> {
    /// Iterate over all cached message entries of a channel.
    ///
    /// The items are ordered by message timestamp i.e. most recent to oldest.
    pub async fn channel_messages(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Message<'static>>>> {
        let mut conn = self.cache.connection().await?;

        let key = RedisKey::ChannelMessages {
            channel: channel_id,
        };
        let ids: Vec<u64> = Cmd::zrange(key, 0, -1)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::Redis)?;

        let keys: Vec<_> = convert_ids_vec(ids)
            .into_iter()
            .map(Id::<MessageMarker>::into)
            .collect();

        self.iter_by_keys(&keys, Some(&mut conn))
            .await
            .map(CacheIter::new)
    }

    /// Iterate over all cached channel entries.
    pub async fn channels(&self) -> CacheResult<CacheIter<Archived<C::Channel<'static>>>> {
        self.iter_all(RedisKey::Channels, Id::<ChannelMarker>::into)
            .await
    }

    /// Iterate over the cached channel entry for each given id.
    pub async fn channels_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Channel<'static>>>>
    where
        I: IntoIterator<Item = Id<ChannelMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached emoji entries.
    pub async fn emojis(&self) -> CacheResult<CacheIter<Archived<C::Emoji<'static>>>> {
        self.iter_all(RedisKey::Emojis, Id::<EmojiMarker>::into)
            .await
    }

    /// Iterate over the cached emoji entry for each given id.
    pub async fn emojis_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Emoji<'static>>>>
    where
        I: IntoIterator<Item = Id<EmojiMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached guild entries.
    pub async fn guilds(&self) -> CacheResult<CacheIter<Archived<C::Guild<'static>>>> {
        self.iter_all(RedisKey::Guilds, Id::<GuildMarker>::into)
            .await
    }

    /// Iterate over the cached guild entry for each given id.
    pub async fn guilds_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Guild<'static>>>>
    where
        I: IntoIterator<Item = Id<GuildMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached message entries.
    pub async fn messages(&self) -> CacheResult<CacheIter<Archived<C::Message<'static>>>> {
        self.iter_all(RedisKey::Messages, Id::<MessageMarker>::into)
            .await
    }

    /// Iterate over the cached message entry for each given id.
    pub async fn messages_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Message<'static>>>>
    where
        I: IntoIterator<Item = Id<MessageMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached role entries.
    pub async fn roles(&self) -> CacheResult<CacheIter<Archived<C::Role<'static>>>> {
        self.iter_all(RedisKey::Roles, Id::<RoleMarker>::into).await
    }

    /// Iterate over the cached role entry for each given id.
    pub async fn roles_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Role<'static>>>>
    where
        I: IntoIterator<Item = Id<RoleMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached stage instance entries.
    pub async fn stage_instances(
        &self,
    ) -> CacheResult<CacheIter<Archived<C::StageInstance<'static>>>> {
        self.iter_all(RedisKey::StageInstances, Id::<StageMarker>::into)
            .await
    }

    /// Iterate over the cached stage instance entry for each given id.
    pub async fn stage_instances_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::StageInstance<'static>>>>
    where
        I: IntoIterator<Item = Id<StageMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached sticker entries.
    pub async fn stickers(&self) -> CacheResult<CacheIter<Archived<C::Sticker<'static>>>> {
        self.iter_all(RedisKey::Stickers, Id::<StickerMarker>::into)
            .await
    }

    /// Iterate over the cached sticker entry for each given id.
    pub async fn stickers_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Sticker<'static>>>>
    where
        I: IntoIterator<Item = Id<StickerMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached user entries.
    pub async fn users(&self) -> CacheResult<CacheIter<Archived<C::User<'static>>>> {
        self.iter_all(RedisKey::Users, Id::<UserMarker>::into).await
    }

    /// Iterate over the cached user entry for each given id.
    pub async fn users_by_ids<I>(
        &self,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::User<'static>>>>
    where
        I: IntoIterator<Item = Id<UserMarker>>,
    {
        self.iter_by_ids(ids).await
    }

    /// Iterate over all cached channel entries of a guild.
    pub async fn guild_channels(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Channel<'static>>>> {
        let key = RedisKey::GuildChannels { id: guild_id };

        self.iter_all(key, Id::<ChannelMarker>::into).await
    }

    /// Iterate over all cached emoji entries of a guild.
    pub async fn guild_emojis(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Emoji<'static>>>> {
        let key = RedisKey::GuildEmojis { id: guild_id };

        self.iter_all(key, Id::<EmojiMarker>::into).await
    }

    /// Iterate over all cached integration entries of a guild.
    pub async fn guild_integrations(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Integration<'static>>>> {
        let key = RedisKey::GuildIntegrations { id: guild_id };

        let key_fn = move |id| RedisKey::Integration {
            guild: guild_id,
            id,
        };

        self.iter_all(key, key_fn).await
    }

    /// Iterate over the cached guild integration entry for each given id.
    pub async fn guild_integrations_by_ids<I>(
        &self,
        guild_id: Id<GuildMarker>,
        ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Integration<'static>>>>
    where
        I: IntoIterator<Item = Id<IntegrationMarker>>,
    {
        let keys: Vec<_> = ids
            .into_iter()
            .map(|id| RedisKey::Integration {
                guild: guild_id,
                id,
            })
            .collect();

        self.iter_by_keys(&keys, None).await
    }

    /// Iterate over all cached member entries of a guild.
    pub async fn guild_members(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Member<'static>>>> {
        let key = RedisKey::GuildMembers { id: guild_id };

        let key_fn = move |user| RedisKey::Member {
            guild: guild_id,
            user,
        };

        self.iter_all(key, key_fn).await
    }

    /// Iterate over the cached guild member entry for each given user id.
    pub async fn guild_members_by_ids<I>(
        &self,
        guild_id: Id<GuildMarker>,
        user_ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Member<'static>>>>
    where
        I: IntoIterator<Item = Id<UserMarker>>,
    {
        let keys: Vec<_> = user_ids
            .into_iter()
            .map(|user| RedisKey::Member {
                guild: guild_id,
                user,
            })
            .collect();

        self.iter_by_keys(&keys, None).await
    }

    /// Iterate over all cached presence entries of a guild.
    pub async fn guild_presences(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Presence<'static>>>> {
        let key = RedisKey::GuildPresences { id: guild_id };

        let key_fn = move |user| RedisKey::Presence {
            guild: guild_id,
            user,
        };

        self.iter_all(key, key_fn).await
    }

    /// Iterate over the cached guild presence entry for each given user id.
    pub async fn guild_presences_by_ids<I>(
        &self,
        guild_id: Id<GuildMarker>,
        user_ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::Presence<'static>>>>
    where
        I: IntoIterator<Item = Id<UserMarker>>,
    {
        let keys: Vec<_> = user_ids
            .into_iter()
            .map(|user| RedisKey::Presence {
                guild: guild_id,
                user,
            })
            .collect();

        self.iter_by_keys(&keys, None).await
    }

    /// Iterate over all cached role entries of a guild.
    pub async fn guild_roles(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Role<'static>>>> {
        let key = RedisKey::GuildRoles { id: guild_id };

        self.iter_all(key, Id::<RoleMarker>::into).await
    }

    /// Iterate over all cached stage instance entries of a guild.
    pub async fn guild_stage_instances(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::StageInstance<'static>>>> {
        let key = RedisKey::GuildStageInstances { id: guild_id };

        self.iter_all(key, Id::<StageMarker>::into).await
    }

    /// Iterate over all cached sticker entries of a guild.
    pub async fn guild_stickers(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::Sticker<'static>>>> {
        let key = RedisKey::GuildStickers { id: guild_id };

        self.iter_all(key, Id::<StickerMarker>::into).await
    }

    /// Iterate over all cached voice state entries of a guild.
    pub async fn guild_voice_states(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<CacheIter<Archived<C::VoiceState<'static>>>> {
        let key = RedisKey::GuildVoiceStates { id: guild_id };

        let key_fn = move |user| RedisKey::VoiceState {
            guild: guild_id,
            user,
        };

        self.iter_all(key, key_fn).await
    }

    /// Iterate over the cached guild voice state entry for each given user id.
    pub async fn guild_voice_states_by_ids<I>(
        &self,
        guild_id: Id<GuildMarker>,
        user_ids: I,
    ) -> CacheResult<OptionalCacheIter<Archived<C::VoiceState<'static>>>>
    where
        I: IntoIterator<Item = Id<UserMarker>>,
    {
        let keys: Vec<_> = user_ids
            .into_iter()
            .map(|user| RedisKey::VoiceState {
                guild: guild_id,
                user,
            })
            .collect();

        self.iter_by_keys(&keys, None).await
    }

    async fn iter_all<T, M, F>(&self, ids_key: RedisKey, key_fn: F) -> CacheResult<CacheIter<T>>
    where
        T: CheckedArchived,
        F: Fn(Id<M>) -> RedisKey,
    {
        let mut conn = self.cache.connection().await?;

        let ids = RedisCache::<C>::get_ids_static(ids_key, &mut conn).await?;
        let keys: Vec<_> = convert_ids_vec(ids).into_iter().map(key_fn).collect();

        self.iter_by_keys(&keys, Some(&mut conn))
            .await
            .map(CacheIter::new)
    }

    async fn iter_by_ids<I, K, T>(&self, ids: I) -> CacheResult<OptionalCacheIter<T>>
    where
        I: IntoIterator<Item = K>,
        RedisKey: From<K>,
        T: CheckedArchived,
    {
        let keys: Vec<_> = ids.into_iter().map(RedisKey::from).collect();

        self.iter_by_keys(&keys, None).await
    }

    async fn iter_by_keys<'a, T: CheckedArchived>(
        &'a self,
        keys: &[RedisKey],
        conn: Option<&mut Connection<'a>>,
    ) -> CacheResult<OptionalCacheIter<T>> {
        let bytes = if keys.is_empty() {
            Vec::new()
        } else {
            let mut conn_;

            let conn_mut = if let Some(conn) = conn {
                conn
            } else {
                conn_ = self.cache.connection().await?;

                &mut conn_
            };

            Cmd::mget(keys).query_async(conn_mut).await?
        };

        Ok(OptionalCacheIter::new(bytes))
    }
}

impl<C> Clone for RedisCacheIter<'_, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for RedisCacheIter<'_, C> {}
