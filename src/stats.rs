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
    ( #[$meta:meta] $fn:ident,$variant:ident ) => {
        #[$meta]
        pub async fn $fn(&mut self) -> CacheResult<usize> {
            let conn = self.conn.get().await?;

            Cmd::scard(RedisKey::$variant)
                .query_async(conn)
                .await
                .map_err(CacheError::Redis)
        }
    };
    ( Guild: #[$meta:meta] $fn:ident,$variant:ident ) => {
        #[$meta]
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
    impl_stats_fn!(
        #[doc = "Total amount of currently cached channels."]
        channels,
        Channels
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached emojis."]
        emojis,
        Emojis
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached guilds."]
        guilds,
        Guilds
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached messages."]
        messages,
        Messages
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached roles."]
        roles,
        Roles
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached stage instances."]
        stage_instances,
        StageInstances
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached stickers."]
        stickers,
        Stickers
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently unavailable guilds."]
        unavailable_guilds,
        UnavailableGuilds
    );

    impl_stats_fn!(
        #[doc = "Total amount of currently cached users."]
        users,
        Users
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached channels for a guild."]
        guild_channels,
        GuildChannels
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached emojis for a guild."]
        guild_emojis,
        GuildEmojis
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached integrations for a guild."]
        guild_integrations,
        GuildIntegrations
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached members for a guild."]
        guild_members,
        GuildMembers
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached presences for a guild."]
        guild_presences,
        GuildPresences
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached roles for a guild."]
        guild_roles,
        GuildRoles
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached stage instances for a guild."]
        guild_stage_instances,
        GuildStageInstances
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached stickers for a guild."]
        guild_stickers,
        GuildStickers
    );

    impl_stats_fn!(
        Guild:
        #[doc = "Amount of currently cached voice states for a guild."]
        guild_voice_states,
        GuildVoiceStates
    );

    /// Amount of currently cached messages for a channel.
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

    /// Amount of known guilds that a user is in.
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
