mod events;

use std::{
    env,
    error::Error as StdError,
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    sync::OnceLock,
};

use rkyv::{
    option::ArchivedOption,
    ser::serializers::AllocSerializer,
    with::{Map, RefAsBox},
    Archive, Serialize,
};
use twilight_model::{
    channel::Channel,
    gateway::{
        event::Event,
        payload::incoming::{ChannelCreate, ChannelPinsUpdate, UserUpdate},
    },
    id::{
        marker::{ChannelMarker, UserMarker},
        Id,
    },
    user::CurrentUser,
    util::Timestamp,
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedChannel, ICachedCurrentUser, Ignore},
    rkyv_util::{
        id::{IdRkyv, IdRkyvMap},
        util::TimestampRkyv,
    },
    CacheError, CachedValue, RedisCache,
};

use crate::events::*;

#[cfg(feature = "bb8")]
type Pool = bb8_redis::bb8::Pool<bb8_redis::RedisConnectionManager>;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
type Pool = deadpool_redis::Pool;

static POOL: OnceLock<Pool> = OnceLock::new();

pub fn pool() -> Pool {
    fn redis_url() -> String {
        dotenvy::dotenv().unwrap();

        env::var("REDIS_URL").unwrap_or_else(|_| {
            panic!(
                "Integration tests require env variable `REDIS_URL`. \
                You can specify it through a .env file."
            )
        })
    }

    #[cfg(feature = "bb8")]
    let init = || {
        let manager = bb8_redis::RedisConnectionManager::new(redis_url()).unwrap();

        bb8_redis::bb8::Pool::builder().build_unchecked(manager)
    };

    #[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
    let init = || {
        let cfg = deadpool_redis::Config::from_url(redis_url());

        cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .unwrap()
    };

    POOL.get_or_init(init).clone()
}

#[tokio::test]
async fn test_channel() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
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
        ) -> Option<fn(&mut CachedValue<Self>, &ChannelPinsUpdate) -> Result<(), Box<dyn StdError>>>
        {
            let update_fn = |value: &mut CachedValue<Self>, update: &ChannelPinsUpdate| {
                value.update_archive(|pinned| {
                    let last_pin_timestamp =
                        unsafe { &mut pinned.get_unchecked_mut().last_pin_timestamp };

                    *last_pin_timestamp = match update.last_pin_timestamp {
                        Some(new_timestamp) => {
                            ArchivedOption::Some(TimestampRkyv::archive(&new_timestamp))
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
        type Serializer = AllocSerializer<0>;

        fn expire_seconds() -> Option<usize> {
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
                && *kind == other.kind.into()
                && *last_pin_timestamp
                    == other
                        .last_pin_timestamp
                        .as_ref()
                        .map(TimestampRkyv::archive)
                && parent_id.to_id_option() == other.parent_id
        }
    }

    let cache = RedisCache::<Config>::with_pool(pool());

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

#[tokio::test]
async fn test_current_user() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        type Channel<'a> = Ignore;
        type CurrentUser<'a> = CachedCurrentUser<'a>;
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
    struct CachedCurrentUser<'a> {
        #[with(RefAsBox)]
        name: &'a str,
        #[with(IdRkyv)]
        id: Id<UserMarker>,
    }

    impl<'a> ICachedCurrentUser<'a> for CachedCurrentUser<'a> {
        fn from_current_user(current_user: &'a CurrentUser) -> Self {
            Self {
                name: &current_user.name,
                id: current_user.id,
            }
        }
    }

    impl Cacheable for CachedCurrentUser<'_> {
        type Serializer = AllocSerializer<0>;

        fn expire_seconds() -> Option<usize> {
            None
        }
    }

    let cache = RedisCache::<Config>::with_pool(pool());

    let expected = current_user();

    let event = Event::UserUpdate(UserUpdate(expected.clone()));
    cache.update(&event).await?;

    let current_user = cache.current_user().await?.expect("missing current user");

    assert_eq!(current_user.name.as_ref(), expected.name);
    assert_eq!(current_user.id, expected.id);

    Ok(())
}
