#![cfg(feature = "cold_resume")]

use std::{collections::HashMap, iter, time::Duration};

use twilight_gateway::Session;
use twilight_redis::{CacheError, RedisCache};

use crate::pool;

#[tokio::test]
async fn test_cold_resume() -> Result<(), CacheError> {
    let cache = RedisCache::<()>::with_pool(pool());

    let session = Session::new(123, "session_id".to_owned());
    let sessions: HashMap<_, _> = (0..25).zip(iter::once(session).cycle()).collect();

    let duration = Duration::from_secs(3);
    cache.freeze(&sessions, Some(duration)).await?;

    let defrosted = cache.defrost(false).await?;
    assert_eq!(defrosted, Some(sessions));

    tokio::time::sleep(duration + Duration::from_secs(1)).await;

    let defrosted = cache.defrost(true).await?;
    assert_eq!(defrosted, None);

    Ok(())
}
