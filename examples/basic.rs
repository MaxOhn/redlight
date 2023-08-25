use std::time::Duration;

use rkyv::{ser::serializers::AlignedSerializer, with::RefAsBox, AlignedVec, Archive, Serialize};
use twilight_model::{guild::Role, id::Id};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedRole, Ignore},
    RedisCache,
};

struct Config;

impl CacheConfig for Config {
    #[cfg(feature = "metrics")]
    const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(30);

    type Channel<'a> = Ignore;
    type CurrentUser<'a> = Ignore;
    type Emoji<'a> = Ignore;
    type Integration<'a> = Ignore;
    type Guild<'a> = Ignore;
    type Member<'a> = Ignore;
    type Message<'a> = Ignore;
    type Presence<'a> = Ignore;
    type Role<'a> = CachedRole<'a>;
    type StageInstance<'a> = Ignore;
    type Sticker<'a> = Ignore;
    type User<'a> = Ignore;
    type VoiceState<'a> = Ignore;
}

#[derive(Archive, Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
struct CachedRole<'a> {
    #[with(RefAsBox)]
    name: &'a str,
}

impl<'a> ICachedRole<'a> for CachedRole<'a> {
    fn from_role(role: &'a Role) -> Self {
        Self {
            name: role.name.as_str(),
        }
    }
}

impl Cacheable for CachedRole<'_> {
    type Serializer = AlignedSerializer<AlignedVec>;

    fn expire() -> Option<Duration> {
        None
    }
}

#[tokio::main]
async fn main() {
    let cache = RedisCache::<Config>::new("redis://127.0.0.1:6789")
        .await
        .unwrap();

    let value = cache.role(Id::new(123)).await.unwrap().unwrap();

    let _name = value.name.as_ref();
}
