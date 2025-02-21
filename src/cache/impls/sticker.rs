use rkyv::{api::high::to_bytes_in, rancor::Source, ser::writer::Buffer, Archived};
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
    config::{CacheConfig, Cacheable, ICachedSticker, SerializeMany},
    error::{MetaError, MetaErrorKind, SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    rkyv_util::id::IdRkyv,
    util::BytesWrap,
    CacheResult, RedisCache,
};

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

        let mut serializer = C::Sticker::serialize_many();

        let (sticker_entries, sticker_ids) = stickers
            .iter()
            .map(|sticker| {
                let id = sticker.id;
                let key = RedisKey::Sticker { id };
                let sticker = C::Sticker::from_sticker(sticker);

                let bytes = serializer
                    .serialize_next(&sticker)
                    .map_err(|e| SerializeError::new(e, SerializeErrorKind::Sticker))?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesWrap(bytes)), id.get()))
            })
            .collect::<CacheResult<(Vec<(RedisKey, BytesWrap<_>)>, Vec<u64>)>>()?;

        if sticker_entries.is_empty() {
            return Ok(());
        }

        pipe.mset(&sticker_entries, C::Sticker::expire());

        let key = RedisKey::GuildStickers { id: guild_id };
        pipe.sadd(key, sticker_ids.as_slice());

        let key = RedisKey::Stickers;
        pipe.sadd(key, sticker_ids);

        if C::Sticker::expire().is_some() {
            stickers
                .iter()
                .try_for_each(|sticker| {
                    let key = StickerMetaKey {
                        sticker: sticker.id,
                    };

                    StickerMeta { guild: guild_id }.store(pipe, key)
                })
                .map_err(|e| MetaError::new(e, MetaErrorKind::Sticker))?;
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
pub(crate) struct StickerMeta {
    #[rkyv(with = IdRkyv)]
    guild: Id<GuildMarker>,
}

impl IMeta<StickerMetaKey> for StickerMeta {
    type Bytes = [u8; 8];

    fn to_bytes<E: Source>(&self) -> Result<Self::Bytes, E> {
        let mut bytes = [0; 8];
        to_bytes_in(self, Buffer::from(&mut bytes))?;

        Ok(bytes)
    }
}
