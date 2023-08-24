use rkyv::{ser::serializers::BufferSerializer, AlignedBytes, Archived};
use tracing::{instrument, trace};
use twilight_model::{
    channel::message::Sticker,
    id::{
        marker::{GuildMarker, StickerMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, HasArchived, IMeta, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedSticker},
    error::{MetaError, MetaErrorKind, SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    rkyv_util::id::IdRkyv,
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

        let (sticker_entries, sticker_ids) = stickers
            .iter()
            .map(|sticker| {
                let id = sticker.id;
                let key = RedisKey::Sticker { id };
                let sticker = C::Sticker::from_sticker(sticker);

                let bytes =
                    sticker
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError {
                            error: Box::new(e),
                            kind: SerializeErrorKind::Sticker,
                        })?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg<_>), u64>>>()?
            .unzip();

        if sticker_entries.is_empty() {
            return Ok(());
        }

        pipe.mset(&sticker_entries, C::Sticker::expire()).ignore();

        let key = RedisKey::GuildStickers { id: guild_id };
        pipe.sadd(key, sticker_ids.as_slice()).ignore();

        let key = RedisKey::Stickers;
        pipe.sadd(key, sticker_ids).ignore();

        if C::Sticker::expire().is_some() {
            stickers
                .iter()
                .try_for_each(|sticker| {
                    let key = StickerMetaKey {
                        sticker: sticker.id,
                    };

                    StickerMeta { guild: guild_id }.store(pipe, key)
                })
                .map_err(|error| MetaError {
                    error,
                    kind: MetaErrorKind::Sticker,
                })?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct StickerMetaKey {
    sticker: Id<StickerMarker>,
}

impl IMetaKey for StickerMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|sticker| Self { sticker })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::Stickers;
        pipe.srem(key, self.sticker.get()).ignore();
    }
}

impl HasArchived for StickerMetaKey {
    type Meta = StickerMeta;

    fn redis_key(&self) -> RedisKey {
        RedisKey::StickerMeta { id: self.sticker }
    }

    fn handle_archived(&self, pipe: &mut Pipeline, archived: &Archived<Self::Meta>) {
        let key = RedisKey::GuildStickers {
            id: archived.guild.into(),
        };

        pipe.srem(key, self.sticker.get());
    }
}

#[derive(rkyv::Archive, rkyv::Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub(crate) struct StickerMeta {
    #[with(IdRkyv)]
    guild: Id<GuildMarker>,
}

impl IMeta<StickerMetaKey> for StickerMeta {
    type Serializer = BufferSerializer<AlignedBytes<8>>;
}
