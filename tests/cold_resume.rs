#![cfg(feature = "cold_resume")]

use std::{collections::HashMap, iter, time::Duration};

use serial_test::serial;
use twilight_gateway::Session;
use twilight_redis::{
    config::{CacheConfig, Ignore},
    CacheError, RedisCache,
};

use crate::pool;

#[tokio::test]
#[serial]
async fn test_cold_resume() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        #[cfg(feature = "metrics")]
        const METRICS_INTERVAL_DURATION: std::time::Duration = std::time::Duration::from_secs(60);

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
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    let cache = RedisCache::<Config>::with_pool(pool()).await?;

    let session = Session::new(123, "session_id".to_owned());
    let sessions: HashMap<_, _> = (0..25).zip(iter::once(session).cycle()).collect();

    let duration = Duration::from_secs(2);
    cache.freeze(&sessions, Some(duration)).await?;

    let defrosted = cache.defrost(false).await?;
    assert_eq!(defrosted, Some(sessions));

    tokio::time::sleep(duration + Duration::from_secs(1)).await;

    let defrosted = cache.defrost(true).await?;
    assert_eq!(defrosted, None);

    Ok(())
}
