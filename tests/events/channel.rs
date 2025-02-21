use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    time::Duration,
};

use redlight::{
    config::{CacheConfig, Cacheable, ICachedChannel, Ignore},
    error::CacheError,
    rkyv_util::{
        id::{IdRkyv, IdRkyvMap},
        timestamp::{ArchivedTimestamp, TimestampRkyv},
    },
    CachedArchive, RedisCache,
};
use rkyv::{
    option::ArchivedOption,
    rancor::Source,
    util::AlignedVec,
    with::{InlineAsBox, Map},
    Archive, Archived, Serialize,
};
use twilight_model::{
    channel::{Channel, ChannelFlags, ChannelType, VideoQualityMode},
    gateway::{
        event::Event,
        payload::incoming::{ChannelCreate, ChannelPinsUpdate},
    },
    id::{marker::ChannelMarker, Id},
    util::{ImageHash, Timestamp},
};

use crate::pool;

#[tokio::test]
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
        type ScheduledEvent<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    struct CachedChannel<'a> {
        #[rkyv(with = Map<InlineAsBox>)]
        name: Option<&'a str>,
        #[rkyv(with = IdRkyv)]
        id: Id<ChannelMarker>,
        kind: u8,
        #[rkyv(with = Map<TimestampRkyv>)]
        last_pin_timestamp: Option<Timestamp>,
        #[rkyv(with = IdRkyvMap)]
        parent_id: Option<Id<ChannelMarker>>,
    }

    impl Debug for ArchivedCachedChannel<'_> {
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

        fn on_pins_update<E: Source>(
        ) -> Option<fn(&mut CachedArchive<Archived<Self>>, &ChannelPinsUpdate) -> Result<(), E>>
        {
            Some(|value, update| {
                value
                    .update_archive(|sealed| {
                        if let Some(new_timestamp) = update.last_pin_timestamp {
                            rkyv::munge::munge! {
                                let ArchivedCachedChannel { last_pin_timestamp, .. } = sealed
                            };

                            // Cannot mutate from `Some` to `None` or vice versa so we
                            // just update `Some` values
                            if let Some(mut last_pin_timestamp) =
                                ArchivedOption::as_seal(last_pin_timestamp)
                            {
                                *last_pin_timestamp = ArchivedTimestamp::new(&new_timestamp);
                            }
                        }
                    })
                    .map_err(Source::new)
            })
        }
    }

    impl Cacheable for CachedChannel<'_> {
        type Bytes = AlignedVec<8>;

        fn expire() -> Option<Duration> {
            None
        }

        fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
            rkyv::api::high::to_bytes_in(self, AlignedVec::<8>::new())
        }

        // we don't update by deserializing so a `serialize_into` impl is not
        // necessary
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
                        .map(ArchivedTimestamp::new)
                && parent_id.as_ref().copied().map(Id::from) == other.parent_id
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
        last_pin_timestamp: Some(Timestamp::parse("2021-01-01T01:01:01+00:00").unwrap()),
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
