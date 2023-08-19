mod async_iter;

use itoa::Buffer;
use twilight_model::id::{marker::GuildMarker, Id};

use crate::{config::CacheConfig, key::RedisKey, CacheResult, RedisCache};

pub use self::async_iter::AsyncIter;

/// Base type to create iterators for cached entries.
pub struct RedisCacheIter<'c, C> {
    cache: &'c RedisCache<C>,
}

macro_rules! def_getter {
    ( $fn:ident, $ret:ident, $variant:ident, $prefix:ident ) => {
        pub async fn $fn(self) -> CacheResult<AsyncIter<'c, C::$ret<'static>>> {
            let mut conn = self.cache.connection().await?;
            let ids: Vec<u64> =
                RedisCache::<C>::get_ids_static(RedisKey::$variant, &mut conn).await?;

            let key_prefix = key_prefix_simple(RedisKey::$prefix);
            let iter = AsyncIter::new(conn, ids, key_prefix);

            Ok(iter)
        }
    };
    ( Guild: $fn:ident, $ret:ident, $variant:ident, $prefix:ident ) => {
        pub async fn $fn(
            self,
            guild_id: Id<GuildMarker>,
        ) -> CacheResult<AsyncIter<'c, C::$ret<'static>>> {
            let mut conn = self.cache.connection().await?;

            let ids: Vec<u64> =
                RedisCache::<C>::get_ids_static(RedisKey::$variant { id: guild_id }, &mut conn)
                    .await?;

            let (key_prefix, buf) = key_prefix_buffered(RedisKey::$prefix, guild_id);
            let iter = AsyncIter::new_with_buf(conn, ids, key_prefix, buf);

            Ok(iter)
        }
    };
}

impl<'c, C> RedisCacheIter<'c, C> {
    pub(crate) fn new(cache: &'c RedisCache<C>) -> Self {
        Self { cache }
    }

    /// Reference to the underlying cache.
    pub fn cache_ref(&self) -> &RedisCache<C> {
        self.cache
    }
}

#[rustfmt::skip]
impl<'c, C: CacheConfig> RedisCacheIter<'c, C> {
    // TODO: docs

    def_getter!(channels, Channel, Channels, CHANNEL_PREFIX);
    def_getter!(emojis, Emoji, Emojis, EMOJI_PREFIX);
    def_getter!(guilds, Guild, Guilds, GUILD_PREFIX);
    def_getter!(roles, Role, Roles, ROLE_PREFIX);
    def_getter!(stage_instances, StageInstance, StageInstances, STAGE_INSTANCE_PREFIX);
    def_getter!(stickers, Sticker, Stickers, STICKER_PREFIX);
    def_getter!(users, User, Users, USER_PREFIX);

    def_getter!(Guild: guild_channels, Channel, GuildChannels, CHANNEL_PREFIX);
    def_getter!(Guild: guild_emojis, Emoji, GuildEmojis, EMOJI_PREFIX);
    def_getter!(Guild: guild_integrations, Integration, GuildIntegrations, INTEGRATION_PREFIX);
    def_getter!(Guild: guild_members, Member, GuildMembers, MEMBER_PREFIX);
    def_getter!(Guild: guild_presences, Presence, GuildPresences, PRESENCE_PREFIX);
    def_getter!(Guild: guild_roles, Role, GuildRoles, ROLE_PREFIX);
    def_getter!(Guild: guild_stage_instances, StageInstance, GuildStageInstances, STAGE_INSTANCE_PREFIX);
    def_getter!(Guild: guild_stickers, Sticker, GuildStickers, STICKER_PREFIX);
    def_getter!(Guild: guild_voice_states, VoiceState, GuildVoiceStates, VOICE_STATE_PREFIX);
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

    let mut key_prefix = Vec::with_capacity(prefix.len() + guild_id.len() + 1);
    key_prefix.extend_from_slice(prefix);
    key_prefix.push(b':');
    key_prefix.extend_from_slice(guild_id.as_bytes());
    key_prefix.push(b':');

    (key_prefix, buf)
}
