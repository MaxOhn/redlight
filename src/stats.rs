use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, UserMarker},
    Id,
};

use crate::{
    key::RedisKey,
    redis::{Cmd, Connection},
    CacheError, CacheResult, RedisCache,
};

/// Retrieve the size count of various cached collections.
///
/// Created via [`RedisCache::stats`].
pub struct RedisCacheStats<'c, C> {
    conn: ConnectionState<'c, C>,
}

macro_rules! impl_stats_fn {
    ($fn:ident,$variant:ident) => {
        pub async fn $fn(&mut self) -> CacheResult<usize> {
            let conn = self.conn.get().await?;

            Cmd::scard(RedisKey::$variant)
                .query_async(conn)
                .await
                .map_err(CacheError::Redis)
        }
    };
    (Guild: $fn:ident,$variant:ident) => {
        pub async fn $fn(&mut self, guild_id: Id<GuildMarker>) -> CacheResult<usize> {
            let conn = self.conn.get().await?;

            Cmd::scard(RedisKey::$variant { id: guild_id })
                .query_async(conn)
                .await
                .map_err(CacheError::Redis)
        }
    };
}

impl<'c, C> RedisCacheStats<'c, C> {
    pub(crate) fn new(cache: &'c RedisCache<C>) -> RedisCacheStats<'c, C> {
        Self {
            conn: ConnectionState::new(cache),
        }
    }
}

impl<C> RedisCacheStats<'_, C> {
    impl_stats_fn!(channels, Channels);
    impl_stats_fn!(emojis, Emojis);
    impl_stats_fn!(guilds, Guilds);
    impl_stats_fn!(messages, Messages);
    impl_stats_fn!(roles, Roles);
    impl_stats_fn!(stage_instances, StageInstances);
    impl_stats_fn!(stickers, Stickers);
    impl_stats_fn!(unavailable_guilds, UnavailableGuilds);
    impl_stats_fn!(users, Users);

    impl_stats_fn!(Guild: guild_channels, GuildChannels);
    impl_stats_fn!(Guild: guild_emojis, GuildEmojis);
    impl_stats_fn!(Guild: guild_integrations, GuildIntegrations);
    impl_stats_fn!(Guild: guild_members, GuildMembers);
    impl_stats_fn!(Guild: guild_presences, GuildPresences);
    impl_stats_fn!(Guild: guild_roles, GuildRoles);
    impl_stats_fn!(Guild: guild_stage_instances, GuildStageInstances);
    impl_stats_fn!(Guild: guild_stickers, GuildStickers);
    impl_stats_fn!(Guild: guild_voice_states, GuildVoiceStates);

    pub async fn channel_messages(&mut self, channel_id: Id<ChannelMarker>) -> CacheResult<usize> {
        let conn = self.conn.get().await?;

        let key = RedisKey::ChannelMessages {
            channel: channel_id,
        };

        Cmd::scard(key)
            .query_async(conn)
            .await
            .map_err(CacheError::Redis)
    }

    pub async fn common_guilds(&mut self, user_id: Id<UserMarker>) -> CacheResult<usize> {
        let conn = self.conn.get().await?;

        Cmd::scard(RedisKey::UserGuilds { id: user_id })
            .query_async(conn)
            .await
            .map_err(CacheError::Redis)
    }
}

enum ConnectionState<'c, C> {
    Cache(&'c RedisCache<C>),
    Connection(Connection<'c>),
}

impl<'c, C> ConnectionState<'c, C> {
    fn new(cache: &'c RedisCache<C>) -> Self {
        Self::Cache(cache)
    }

    async fn get(&mut self) -> CacheResult<&mut Connection<'c>> {
        match self {
            ConnectionState::Cache(cache) => {
                let conn = cache.connection().await?;
                *self = Self::Connection(conn);

                let Self::Connection(conn) = self else {
                    // SAFETY: we just assigned a connection
                    unsafe { std::hint::unreachable_unchecked() }
                };

                Ok(conn)
            }
            ConnectionState::Connection(conn) => Ok(conn),
        }
    }
}
