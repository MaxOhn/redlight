use tracing::{instrument, trace};
use twilight_model::{
    guild::Emoji,
    id::{marker::GuildMarker, Id},
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedEmoji},
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type EmojiSerializer<'a, C> = <<C as CacheConfig>::Emoji<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
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

                trace!(bytes = bytes.len());

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
}
