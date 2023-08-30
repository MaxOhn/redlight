mod async_iter;

use itoa::Buffer;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};

use crate::{
    config::{CacheConfig, Cacheable},
    key::RedisKey,
    redis::Cmd,
    CacheError, CacheResult, RedisCache,
};

pub use self::async_iter::AsyncIter;

/// Base type to create iterators for cached entries.
///
/// The iteration order of all iterators is arbitrary, except for [`RedisCacheIter::channel_messages`]
/// whose order is the message timestamp i.e. from most recent to oldest.
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

impl<'c, C: CacheConfig> RedisCacheIter<'c, C> {
    /// Iterate over all cached channel entries.
    pub async fn channels(self) -> CacheResult<AsyncIter<'c, C::Channel<'static>>> {
        self.iter_all(RedisKey::Channels, RedisKey::CHANNEL_PREFIX)
            .await
    }

    /// Iterate over all cached message entries of a channel.
    ///
    /// The items are ordered by message timestamp i.e. most recent to oldest.
    pub async fn channel_messages(
        self,
        channel_id: Id<ChannelMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Message<'static>>> {
        let key = RedisKey::ChannelMessages {
            channel: channel_id,
        };

        let mut conn = self.cache.connection().await?;

        let ids: Vec<u64> = Cmd::zrange(key, 0, -1)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::Redis)?;

        let key_prefix = key_prefix_simple(RedisKey::MESSAGE_PREFIX);
        let iter = AsyncIter::new(conn, ids, key_prefix);

        Ok(iter)
    }

    /// Iterate over all cached emoji entries.
    pub async fn emojis(self) -> CacheResult<AsyncIter<'c, C::Emoji<'static>>> {
        self.iter_all(RedisKey::Emojis, RedisKey::EMOJI_PREFIX)
            .await
    }

    /// Iterate over all cached guild entries.
    pub async fn guilds(self) -> CacheResult<AsyncIter<'c, C::Guild<'static>>> {
        self.iter_all(RedisKey::Guilds, RedisKey::GUILD_PREFIX)
            .await
    }

    /// Iterate over all cached message entries.
    pub async fn messages(self) -> CacheResult<AsyncIter<'c, C::Message<'static>>> {
        self.iter_all(RedisKey::Messages, RedisKey::MESSAGE_PREFIX)
            .await
    }

    /// Iterate over all cached role entries.
    pub async fn roles(self) -> CacheResult<AsyncIter<'c, C::Role<'static>>> {
        self.iter_all(RedisKey::Roles, RedisKey::ROLE_PREFIX).await
    }

    /// Iterate over all cached stage instance entries.
    pub async fn stage_instances(self) -> CacheResult<AsyncIter<'c, C::StageInstance<'static>>> {
        self.iter_all(RedisKey::StageInstances, RedisKey::STAGE_INSTANCE_PREFIX)
            .await
    }

    /// Iterate over all cached sticker entries.
    pub async fn stickers(self) -> CacheResult<AsyncIter<'c, C::Sticker<'static>>> {
        self.iter_all(RedisKey::Stickers, RedisKey::STICKER_PREFIX)
            .await
    }

    /// Iterate over all cached user entries.
    pub async fn users(self) -> CacheResult<AsyncIter<'c, C::User<'static>>> {
        self.iter_all(RedisKey::Users, RedisKey::USER_PREFIX).await
    }

    /// Iterate over all cached channel entries of a guild.
    pub async fn guild_channels(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Channel<'static>>> {
        let key = RedisKey::GuildChannels { id: guild_id };

        self.iter_guild_simple(key, RedisKey::CHANNEL_PREFIX).await
    }

    /// Iterate over all cached emoji entries of a guild.
    pub async fn guild_emojis(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Emoji<'static>>> {
        let key = RedisKey::GuildEmojis { id: guild_id };

        self.iter_guild_simple(key, RedisKey::EMOJI_PREFIX).await
    }

    /// Iterate over all cached integration entries of a guild.
    pub async fn guild_integrations(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Integration<'static>>> {
        let key = RedisKey::GuildIntegrations { id: guild_id };

        self.iter_guild_buffered(guild_id, key, RedisKey::INTEGRATION_PREFIX)
            .await
    }

    /// Iterate over all cached member entries of a guild.
    pub async fn guild_members(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Member<'static>>> {
        let key = RedisKey::GuildMembers { id: guild_id };

        self.iter_guild_buffered(guild_id, key, RedisKey::MEMBER_PREFIX)
            .await
    }

    /// Iterate over all cached presence entries of a guild.
    pub async fn guild_presences(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Presence<'static>>> {
        let key = RedisKey::GuildPresences { id: guild_id };

        self.iter_guild_buffered(guild_id, key, RedisKey::PRESENCE_PREFIX)
            .await
    }

    /// Iterate over all cached role entries of a guild.
    pub async fn guild_roles(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Role<'static>>> {
        let key = RedisKey::GuildRoles { id: guild_id };

        self.iter_guild_simple(key, RedisKey::ROLE_PREFIX).await
    }

    /// Iterate over all cached stage instance entries of a guild.
    pub async fn guild_stage_instances(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::StageInstance<'static>>> {
        let key = RedisKey::GuildStageInstances { id: guild_id };

        self.iter_guild_simple(key, RedisKey::STAGE_INSTANCE_PREFIX)
            .await
    }

    /// Iterate over all cached sticker entries of a guild.
    pub async fn guild_stickers(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::Sticker<'static>>> {
        let key = RedisKey::GuildStickers { id: guild_id };

        self.iter_guild_simple(key, RedisKey::STICKER_PREFIX).await
    }

    /// Iterate over all cached voice state entries of a guild.
    pub async fn guild_voice_states(
        self,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<AsyncIter<'c, C::VoiceState<'static>>> {
        let key = RedisKey::GuildVoiceStates { id: guild_id };

        self.iter_guild_buffered(guild_id, key, RedisKey::VOICE_STATE_PREFIX)
            .await
    }

    async fn iter_all<T: Cacheable>(
        self,
        key: RedisKey,
        prefix: &'static [u8],
    ) -> CacheResult<AsyncIter<'c, T>> {
        let mut conn = self.cache.connection().await?;

        let ids: Vec<u64> = RedisCache::<C>::get_ids_static(key, &mut conn).await?;

        let key_prefix = key_prefix_simple(prefix);
        let iter = AsyncIter::new(conn, ids, key_prefix);

        Ok(iter)
    }

    async fn iter_guild_simple<T: Cacheable>(
        self,
        key: RedisKey,
        prefix: &'static [u8],
    ) -> CacheResult<AsyncIter<'c, T>> {
        let mut conn = self.cache.connection().await?;

        let ids: Vec<u64> = RedisCache::<C>::get_ids_static(key, &mut conn).await?;

        let key_prefix = key_prefix_simple(prefix);
        let iter = AsyncIter::new(conn, ids, key_prefix);

        Ok(iter)
    }

    async fn iter_guild_buffered<T: Cacheable>(
        self,
        guild_id: Id<GuildMarker>,
        key: RedisKey,
        prefix: &'static [u8],
    ) -> CacheResult<AsyncIter<'c, T>> {
        let mut conn = self.cache.connection().await?;

        let ids: Vec<u64> = RedisCache::<C>::get_ids_static(key, &mut conn).await?;

        let (key_prefix, buf) = key_prefix_buffered(prefix, guild_id);
        let iter = AsyncIter::new_with_buf(conn, ids, key_prefix, buf);

        Ok(iter)
    }
}

impl<'c, C> Clone for RedisCacheIter<'c, C> {
    fn clone(&self) -> Self {
        Self { cache: self.cache }
    }
}

impl<'c, C> Copy for RedisCacheIter<'c, C> {}

fn key_prefix_simple(prefix: &'static [u8]) -> Vec<u8> {
    let mut key_prefix = Vec::with_capacity(prefix.len() + 1);
    key_prefix.extend_from_slice(prefix);
    key_prefix.push(b':');

    key_prefix
}

fn key_prefix_buffered(prefix: &'static [u8], guild_id: Id<GuildMarker>) -> (Vec<u8>, Buffer) {
    let mut buf = Buffer::new();
    let guild_id = buf.format(guild_id.get());

    let mut key_prefix = Vec::with_capacity(prefix.len() + 1 + 2 * (guild_id.len() + 1));
    key_prefix.extend_from_slice(prefix);
    key_prefix.push(b':');
    key_prefix.extend_from_slice(guild_id.as_bytes());
    key_prefix.push(b':');

    (key_prefix, buf)
}
