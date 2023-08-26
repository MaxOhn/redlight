use std::{
    error::Error as StdError,
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    time::Duration,
};

use rkyv::{
    option::ArchivedOption,
    ser::serializers::AlignedSerializer,
    with::{Map, RefAsBox},
    AlignedVec, Archive, Serialize,
};
use serial_test::serial;
use twilight_model::{
    channel::{Channel, ChannelFlags, ChannelType, VideoQualityMode},
    gateway::{
        event::Event,
        payload::incoming::{ChannelCreate, ChannelPinsUpdate},
    },
    id::{marker::ChannelMarker, Id},
    util::{ImageHash, Timestamp},
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedChannel, Ignore},
    rkyv_util::{
        id::{IdRkyv, IdRkyvMap},
        util::TimestampRkyv,
    },
    CacheError, CachedArchive, RedisCache,
};

use crate::pool;

#[tokio::test]
#[serial]
async fn test_channel() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        #[cfg(feature = "metrics")]
        const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(60);

        type Channel<'a> = CachedChannel<'a>;
        type CurrentUser<'a> = Ignore;
        type Emoji<'a> = Ignore;
        type Guild<'a> = Ignore;
        type Integration<'a> = Ignore;
        type Member<'a> = Ignore;
        type Message<'a> = Ignore;
        type Presence<'a> = Ignore;
        type Role<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedChannel<'a> {
        #[with(Map<RefAsBox>)]
        name: Option<&'a str>,
        #[with(IdRkyv)]
        id: Id<ChannelMarker>,
        kind: u8,
        #[with(Map<TimestampRkyv>)]
        last_pin_timestamp: Option<Timestamp>,
        #[with(IdRkyvMap)]
        parent_id: Option<Id<ChannelMarker>>,
    }

    impl<'a> Debug for ArchivedCachedChannel<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            f.debug_struct("ArchivedCachedChannel")
                .field("name", &self.name.as_deref())
                .field("id", &self.id)
                .field("kind", &self.kind)
                .field("last_pin_timestamp", &self.last_pin_timestamp)
                .field("parent_id", &self.parent_id)
                .finish()
        }
    }

    impl<'a> ICachedChannel<'a> for CachedChannel<'a> {
        fn from_channel(channel: &'a Channel) -> Self {
            Self {
                name: channel.name.as_deref(),
                id: channel.id,
                kind: channel.kind.into(),
                last_pin_timestamp: channel.last_pin_timestamp,
                parent_id: channel.parent_id,
            }
        }

        fn on_pins_update(
        ) -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), Box<dyn StdError>>>
        {
            let update_fn = |value: &mut CachedArchive<Self>, update: &ChannelPinsUpdate| {
                value.update_archive(|pinned| {
                    let last_pin_timestamp =
                        unsafe { &mut pinned.get_unchecked_mut().last_pin_timestamp };

                    *last_pin_timestamp = match update.last_pin_timestamp {
                        Some(new_timestamp) => {
                            ArchivedOption::Some(TimestampRkyv::archive(&new_timestamp).into())
                        }
                        None => ArchivedOption::None,
                    };
                });

                Ok(())
            };

            Some(update_fn)
        }
    }

    impl Cacheable for CachedChannel<'_> {
        type Serializer = AlignedSerializer<AlignedVec>;

        fn expire() -> Option<Duration> {
            None
        }
    }

    impl PartialEq<Channel> for ArchivedCachedChannel<'_> {
        fn eq(&self, other: &Channel) -> bool {
            let Self {
                name,
                id,
                kind,
                last_pin_timestamp,
                parent_id,
            } = self;

            name.as_deref() == other.name.as_deref()
                && *id == other.id
                && *kind == u8::from(other.kind)
                && *last_pin_timestamp
                    == other
                        .last_pin_timestamp
                        .as_ref()
                        .map(TimestampRkyv::archive)
                && parent_id.to_id_option() == other.parent_id
        }
    }

    let cache = RedisCache::<Config>::new_with_pool(pool()).await?;

    let mut expected = text_channel();

    let event = Event::ChannelCreate(Box::new(ChannelCreate(expected.clone())));
    cache.update(&event).await?;

    let channel = cache.channel(expected.id).await?.expect("missing channel");

    assert_eq!(channel.deref(), &expected);

    let update = channel_pins_update();
    assert_ne!(expected.last_pin_timestamp, update.last_pin_timestamp);
    expected.last_pin_timestamp = update.last_pin_timestamp;

    let event = Event::ChannelPinsUpdate(update);
    cache.update(&event).await?;

    let channel = cache.channel(expected.id).await?.expect("missing channel");

    assert_eq!(channel.deref(), &expected);

    Ok(())
}

pub fn text_channel() -> Channel {
    Channel {
        application_id: None,
        applied_tags: None,
        available_tags: None,
        bitrate: None,
        default_auto_archive_duration: None,
        default_forum_layout: None,
        default_reaction_emoji: None,
        default_sort_order: None,
        default_thread_rate_limit_per_user: None,
        flags: Some(ChannelFlags::PINNED),
        guild_id: Some(Id::new(898)),
        icon: Some(ImageHash::new([1; 16], false)),
        id: Id::new(765),
        invitable: Some(true),
        kind: ChannelType::GuildText,
        last_message_id: Some(Id::new(111)),
        last_pin_timestamp: None,
        managed: None,
        member: None,
        member_count: None,
        message_count: None,
        name: Some("channel_name".to_owned()),
        newly_created: None,
        nsfw: Some(false),
        owner_id: None,
        parent_id: Some(Id::new(222)),
        permission_overwrites: Some(vec![]),
        position: Some(6),
        rate_limit_per_user: None,
        recipients: None,
        rtc_region: None,
        thread_metadata: None,
        topic: Some("channel_topic".to_owned()),
        user_limit: Some(123),
        video_quality_mode: Some(VideoQualityMode::Auto),
    }
}

pub fn channel_pins_update() -> ChannelPinsUpdate {
    ChannelPinsUpdate {
        channel_id: Id::new(765),
        guild_id: Some(Id::new(898)),
        last_pin_timestamp: Some(Timestamp::parse("2022-02-02T02:02:02+00:00").unwrap()),
    }
}
