use std::time::Duration;

use rkyv::{ser::serializers::BufferSerializer, with::Map, AlignedBytes, Archive, Serialize};
use serial_test::serial;
use twilight_model::{
    gateway::{
        event::Event,
        payload::incoming::PresenceUpdate,
        presence::{ClientStatus, Presence, Status, UserOrId},
    },
    id::{marker::UserMarker, Id},
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedPresence, Ignore},
    rkyv_util::{id::IdRkyv, presence::StatusRkyv},
    CacheError, RedisCache,
};

use crate::pool;

use super::user::user;

#[tokio::test]
#[serial]
async fn test_presence() -> Result<(), CacheError> {
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
        type Presence<'a> = CachedPresence;
        type Role<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedPresence {
        #[with(Map<StatusRkyv>)]
        desktop_status: Option<Status>,
        #[with(IdRkyv)]
        user_id: Id<UserMarker>,
    }

    impl<'a> ICachedPresence<'a> for CachedPresence {
        fn from_presence(presence: &'a Presence) -> Self {
            Self {
                desktop_status: presence.client_status.desktop,
                user_id: presence.user.id(),
            }
        }
    }

    impl Cacheable for CachedPresence {
        type Serializer = BufferSerializer<AlignedBytes<16>>;

        fn expire() -> Option<Duration> {
            None
        }
    }

    let cache = RedisCache::<Config>::new_with_pool(pool()).await?;

    let expected = presence();

    let event = Event::PresenceUpdate(Box::new(PresenceUpdate(expected.clone())));
    cache.update(&event).await?;

    let presence = cache
        .presence(expected.guild_id, expected.user.id())
        .await?
        .expect("missing presence");

    assert_eq!(
        presence.desktop_status.as_ref().copied().map(From::from),
        expected.client_status.desktop
    );
    assert_eq!(presence.user_id, expected.user.id());

    Ok(())
}

pub fn presence() -> Presence {
    Presence {
        activities: Vec::new(),
        client_status: ClientStatus {
            desktop: Some(Status::DoNotDisturb),
            mobile: None,
            web: None,
        },
        guild_id: Id::new(224),
        status: Status::Online,
        user: UserOrId::User(user()),
    }
}
