use tracing::{instrument, trace};
use twilight_model::{
    channel::message::Sticker,
    id::{marker::GuildMarker, Id},
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedSticker},
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type StickerSerializer<'a, C> = <<C as CacheConfig>::Sticker<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
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

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg<_>), u64>>>()?
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
}
