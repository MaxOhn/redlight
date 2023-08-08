use rkyv::{
    ser::serializers::AllocSerializer,
    with::{Map, RefAsBox},
    Archive, Serialize,
};
use twilight_model::{
    gateway::{event::Event, payload::incoming::UserUpdate},
    id::{marker::UserMarker, Id},
    user::{CurrentUser, PremiumType, UserFlags},
    util::ImageHash,
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedCurrentUser, Ignore},
    rkyv_util::{id::IdRkyv, util::ImageHashRkyv},
    CacheError, RedisCache,
};

use crate::pool;

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
        #[with(Map<ImageHashRkyv>)]
        avatar: Option<ImageHash>,
        #[with(RefAsBox)]
        name: &'a str,
        #[with(IdRkyv)]
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