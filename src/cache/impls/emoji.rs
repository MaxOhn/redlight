use rkyv::{ser::serializers::BufferSerializer, AlignedBytes, Archived};
use tracing::{instrument, trace};
use twilight_model::{
    guild::Emoji,
    id::{
        marker::{EmojiMarker, GuildMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, HasArchived, IMeta, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedEmoji},
    error::{MetaError, MetaErrorKind, SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    rkyv_util::id::IdRkyv,
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

        let (emoji_entries, emoji_ids) = emojis
            .iter()
            .map(|emoji| {
                let id = emoji.id;
                let key = RedisKey::Emoji { id };
                let emoji = C::Emoji::from_emoji(emoji);

                let bytes = emoji
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError {
                        error: Box::new(e),
                        kind: SerializeErrorKind::Emoji,
                    })?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg<_>), u64>>>()?
            .unzip();

        if emoji_entries.is_empty() {
            return Ok(());
        }

        pipe.mset(&emoji_entries, C::Emoji::expire());

        let key = RedisKey::GuildEmojis { id: guild_id };
        pipe.sadd(key, emoji_ids.as_slice());

        let key = RedisKey::Emojis;
        pipe.sadd(key, emoji_ids);

        if C::Emoji::expire().is_some() {
            emojis
                .iter()
                .try_for_each(|emoji| {
                    let key = EmojiMetaKey { emoji: emoji.id };

                    EmojiMeta { guild: guild_id }.store(pipe, key)
                })
                .map_err(|error| MetaError {
                    error,
                    kind: MetaErrorKind::Emoji,
                })?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct EmojiMetaKey {
    emoji: Id<EmojiMarker>,
}

impl IMetaKey for EmojiMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|emoji| Self { emoji })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::Emojis;
        pipe.srem(key, self.emoji.get()).ignore();
    }
}

impl HasArchived for EmojiMetaKey {
    type Meta = EmojiMeta;

    fn redis_key(&self) -> RedisKey {
        RedisKey::EmojiMeta { id: self.emoji }
    }

    fn handle_archived(&self, pipe: &mut Pipeline, archived: &Archived<Self::Meta>) {
        let key = RedisKey::GuildEmojis {
            id: archived.guild.into(),
        };
        pipe.srem(key, self.emoji.get());
    }
}

#[derive(rkyv::Archive, rkyv::Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub(crate) struct EmojiMeta {
    #[with(IdRkyv)]
    guild: Id<GuildMarker>,
}

impl IMeta<EmojiMetaKey> for EmojiMeta {
    type Serializer = BufferSerializer<AlignedBytes<8>>;
}
