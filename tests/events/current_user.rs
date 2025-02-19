use std::time::Duration;

use redlight::{
    config::{CacheConfig, Cacheable, ICachedCurrentUser, Ignore},
    error::CacheError,
    rkyv_util::id::IdRkyv,
    RedisCache,
};
use rkyv::{
    rancor::Source,
    util::AlignedVec,
    with::{InlineAsBox, Map},
    Archive, Serialize,
};
use twilight_model::{
    gateway::{event::Event, payload::incoming::UserUpdate},
    id::{marker::UserMarker, Id},
    user::{CurrentUser, PremiumType, UserFlags},
    util::ImageHash,
};

use crate::pool;

#[tokio::test]
async fn test_current_user() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        #[cfg(feature = "metrics")]
        const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(60);

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
    #[rkyv(remote = ImageHash)]
    #[expect(unused)]
    struct ImageHashRkyv {
        #[rkyv(getter = get_animated)]
        animated: bool,
        #[rkyv(getter = get_bytes)]
        bytes: [u8; 16],
    }

    fn get_animated(image_hash: &ImageHash) -> bool {
        image_hash.is_animated()
    }

    fn get_bytes(image_hash: &ImageHash) -> [u8; 16] {
        image_hash.bytes()
    }

    #[derive(Archive, Serialize)]
    struct CachedCurrentUser<'a> {
        #[rkyv(with = Map<ImageHashRkyv>)]
        avatar: Option<ImageHash>,
        #[rkyv(with = InlineAsBox)]
        name: &'a str,
        #[rkyv(with = IdRkyv)]
        id: Id<UserMarker>,
    }

    impl<'a> ICachedCurrentUser<'a> for CachedCurrentUser<'a> {
        fn from_current_user(current_user: &'a CurrentUser) -> Self {
            Self {
                avatar: current_user.avatar,
                name: &current_user.name,
                id: current_user.id,
            }
        }
    }

    impl Cacheable for CachedCurrentUser<'_> {
        type Bytes = AlignedVec;

        fn expire() -> Option<Duration> {
            None
        }

        fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
            rkyv::to_bytes(self)
        }
    }

    let cache = RedisCache::<Config>::new_with_pool(pool()).await?;

    let expected = current_user();

    let event = Event::UserUpdate(UserUpdate(expected.clone()));
    cache.update(&event).await?;

    let current_user = cache.current_user().await?.expect("missing current user");

    assert_eq!(current_user.name.as_ref(), expected.name);
    assert_eq!(current_user.id, expected.id);

    Ok(())
}

pub fn current_user() -> CurrentUser {
    CurrentUser {
        accent_color: Some(234),
        avatar: Some(ImageHash::new([4; 16], true)),
        banner: Some(ImageHash::new([4; 16], false)),
        bot: true,
        discriminator: 2345,
        email: None,
        flags: Some(UserFlags::ACTIVE_DEVELOPER | UserFlags::VERIFIED_DEVELOPER),
        id: Id::new(789),
        locale: Some("en-US".to_owned()),
        mfa_enabled: false,
        name: "current_user".to_owned(),
        premium_type: Some(PremiumType::None),
        public_flags: Some(UserFlags::ACTIVE_DEVELOPER),
        verified: Some(true),
    }
}
