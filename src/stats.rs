use futures_util::future::BoxFuture;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, UserMarker},
    Id,
};

use crate::{
    key::RedisKey,
    redis::{Cmd, Connection},
    CacheError, CacheResult, RedisCache,
};

pub struct RedisCacheStats<'c> {
    conn: ConnectionState<'c>,
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

impl<'c> RedisCacheStats<'c> {
    pub(crate) fn new<C: Send + Sync + 'static>(cache: &'c RedisCache<C>) -> RedisCacheStats<'c> {
        Self {
            conn: ConnectionState::new(cache),
        }
    }
}

impl RedisCacheStats<'_> {
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

enum ConnectionState<'c> {
    Future(BoxFuture<'c, CacheResult<Connection<'c>>>),
    Ready(Connection<'c>),
}

impl<'c> ConnectionState<'c> {
    fn new<C: Send + Sync + 'static>(cache: &'c RedisCache<C>) -> Self {
        Self::Future(Box::pin(cache.connection()))
    }

    async fn get(&mut self) -> CacheResult<&mut Connection<'c>> {
        match self {
            ConnectionState::Future(fut) => {
                *self = Self::Ready(fut.await?);
                let Self::Ready(conn) = self else {
                    unreachable!()
                };

                Ok(conn)
            }
            ConnectionState::Ready(conn) => Ok(conn),
        }
    }
}
