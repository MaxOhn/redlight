use std::time::Duration;

use rkyv::{ser::serializers::BufferSerializer, AlignedBytes, Archive, Serialize};
use serial_test::serial;
use twilight_model::{
    channel::{stage_instance::PrivacyLevel, StageInstance},
    gateway::{event::Event, payload::incoming::StageInstanceCreate},
    id::Id,
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedStageInstance, Ignore},
    rkyv_util::stage_instance::PrivacyLevelRkyv,
    CacheError, RedisCache,
};

use crate::pool;

#[tokio::test]
#[serial]
async fn test_channel() -> Result<(), CacheError> {
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
        type StageInstance<'a> = CachedStageInstance;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedStageInstance {
        #[with(PrivacyLevelRkyv)]
        privacy_level: PrivacyLevel,
    }

    impl<'a> ICachedStageInstance<'a> for CachedStageInstance {
        fn from_stage_instance(stage_instance: &'a StageInstance) -> Self {
            Self {
                privacy_level: stage_instance.privacy_level,
            }
        }
    }

    impl Cacheable for CachedStageInstance {
        type Serializer = BufferSerializer<AlignedBytes<1>>;

        fn expire() -> Option<Duration> {
            None
        }
    }

    let cache = RedisCache::<Config>::with_pool(pool()).await?;

    let expected = stage_instance();

    let event = Event::StageInstanceCreate(StageInstanceCreate(expected.clone()));
    cache.update(&event).await?;

    let stage = cache
        .stage_instance(expected.id)
        .await?
        .expect("missing stage instance");

    assert_eq!(stage.privacy_level, expected.privacy_level as u8);

    Ok(())
}

pub fn stage_instance() -> StageInstance {
    StageInstance {
        channel_id: Id::new(555),
        guild_id: Id::new(556),
        guild_scheduled_event_id: Some(Id::new(557)),
        id: Id::new(558),
        privacy_level: PrivacyLevel::GuildOnly,
        topic: "stage instance topic".to_owned(),
    }
}
