use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    time::Duration,
};

use redlight::{
    config::{CacheConfig, Cacheable, ICachedSticker, Ignore},
    error::CacheError,
    rkyv_util::util::RkyvAsU8,
    RedisCache,
};
use rkyv::{
    rancor::Source,
    util::AlignedVec,
    with::{InlineAsBox, Map},
    Archive, Serialize,
};
use twilight_model::{
    channel::message::{
        sticker::{StickerFormatType, StickerType},
        Sticker,
    },
    gateway::{event::Event, payload::incoming::GuildStickersUpdate},
    id::Id,
};

use crate::pool;

#[tokio::test]
async fn test_stickers() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        #[cfg(feature = "metrics")]
        const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(60);

        type Channel<'a> = Ignore;
        type CurrentUser<'a> = Ignore;
        type Emoji<'a> = Ignore;
        type Guild<'a> = Ignore;
        type Integration<'a> = Ignore;
        type Member<'a> = Ignore;
        type Message<'a> = Ignore;
        type Presence<'a> = Ignore;
        type Role<'a> = Ignore;
        type ScheduledEvent<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = CachedSticker<'a>;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    struct CachedSticker<'a> {
        #[rkyv(with = Map<InlineAsBox>)]
        description: Option<&'a str>,
        #[rkyv(with = RkyvAsU8)]
        format_type: StickerFormatType,
        #[rkyv(with = RkyvAsU8)]
        kind: StickerType,
    }

    impl<'a> ICachedSticker<'a> for CachedSticker<'a> {
        fn from_sticker(sticker: &'a Sticker) -> Self {
            Self {
                description: sticker.description.as_deref(),
                format_type: sticker.format_type,
                kind: sticker.kind,
            }
        }
    }

    impl Cacheable for CachedSticker<'_> {
        type Bytes = AlignedVec;

        fn expire() -> Option<Duration> {
            None
        }

        fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
            rkyv::to_bytes(self)
        }

        // we don't update by deserializing so a `serialize_into` impl is not
        // necessary
    }

    impl PartialEq<Sticker> for ArchivedCachedSticker<'_> {
        fn eq(&self, other: &Sticker) -> bool {
            self.description.as_deref() == other.description.as_deref()
                && self.format_type == u8::from(other.format_type)
                && self.kind == u8::from(other.kind)
        }
    }

    impl Debug for ArchivedCachedSticker<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            f.debug_struct("ArchivedCachedSticker")
                .field("description", &self.description)
                .field("format_type", &StickerFormatType::from(self.format_type))
                .field("kind", &StickerType::from(self.kind))
                .finish()
        }
    }

    let cache = RedisCache::<Config>::new_with_pool(pool()).await?;

    let expected = stickers();
    let guild_id = expected[0].guild_id.unwrap();

    let update = GuildStickersUpdate {
        guild_id,
        stickers: expected.clone(),
    };

    let event = Event::GuildStickersUpdate(update);
    cache.update(&event).await?;

    let sticker = cache
        .sticker(expected[0].id)
        .await?
        .expect("missing sticker");

    assert_eq!(sticker.deref(), &expected[0]);

    let ids = cache.guild_sticker_ids(guild_id).await?;

    assert_eq!(ids.len(), expected.len());

    expected
        .iter()
        .for_each(|sticker| assert!(ids.contains(&sticker.id)));

    let mut count = 0;

    for res in cache.iter().stickers().await? {
        #[cfg(feature = "bytecheck")]
        let archived = res?;

        #[cfg(not(feature = "bytecheck"))]
        let archived = res;

        count += 1;
        assert!(expected.iter().any(|sticker| archived.deref() == sticker));
    }

    assert_eq!(count, expected.len());

    let mut count = 0;

    for res in cache.iter().guild_stickers(guild_id).await? {
        #[cfg(feature = "bytecheck")]
        let archived = res?;

        #[cfg(not(feature = "bytecheck"))]
        let archived = res;

        count += 1;
        assert!(expected.iter().any(|sticker| archived.deref() == sticker));
    }

    assert_eq!(count, expected.len());

    Ok(())
}

pub fn stickers() -> Vec<Sticker> {
    vec![
        Sticker {
            available: true,
            description: Some("sticker description".to_owned()),
            format_type: StickerFormatType::Gif,
            guild_id: Some(Id::new(444)),
            id: Id::new(989),
            kind: StickerType::Guild,
            name: "sticker_name".to_owned(),
            pack_id: None,
            sort_value: None,
            tags: "".to_owned(),
            user: None,
        },
        Sticker {
            available: true,
            description: Some("another sticker description".to_owned()),
            format_type: StickerFormatType::Apng,
            guild_id: Some(Id::new(444)),
            id: Id::new(988),
            kind: StickerType::Guild,
            name: "another_sticker_name".to_owned(),
            pack_id: None,
            sort_value: None,
            tags: "".to_owned(),
            user: None,
        },
    ]
}
