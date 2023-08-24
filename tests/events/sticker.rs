use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    time::Duration,
};

use rkyv::{
    ser::serializers::AlignedSerializer,
    with::{Map, RefAsBox},
    AlignedVec, Archive, Serialize,
};
use serial_test::serial;
use twilight_model::{
    channel::message::{
        sticker::{StickerFormatType, StickerType},
        Sticker,
    },
    gateway::{event::Event, payload::incoming::GuildStickersUpdate},
    id::Id,
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedSticker, Ignore},
    rkyv_util::util::RkyvAsU8,
    CacheError, RedisCache,
};

use crate::pool;

#[tokio::test]
#[serial]
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
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = CachedSticker<'a>;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedSticker<'a> {
        #[with(Map<RefAsBox>)]
        description: Option<&'a str>,
        #[with(RkyvAsU8)]
        format_type: StickerFormatType,
        #[with(RkyvAsU8)]
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
        type Serializer = AlignedSerializer<AlignedVec>;

        fn expire() -> Option<Duration> {
            None
        }
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

    let cache = RedisCache::<Config>::with_pool(pool()).await?;

    let expected = stickers();

    let update = GuildStickersUpdate {
        guild_id: expected[0].guild_id.unwrap(),
        stickers: expected.clone(),
    };

    let event = Event::GuildStickersUpdate(update);
    cache.update(&event).await?;

    let sticker = cache
        .sticker(expected[0].id)
        .await?
        .expect("missing sticker");

    assert_eq!(sticker.deref(), &expected[0]);

    let ids = cache
        .guild_sticker_ids(expected[0].guild_id.unwrap())
        .await?;

    assert_eq!(ids.len(), expected.len());

    expected
        .iter()
        .for_each(|sticker| assert!(ids.contains(&sticker.id)));

    let mut iter = cache.iter().stickers().await?;
    let mut count = 0;

    while let Some(res) = iter.next_item().await {
        let archived = res?;
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
