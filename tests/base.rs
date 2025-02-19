mod cold_resume;
mod events;
mod metrics;

use std::{env, sync::OnceLock};

use tracing::warn;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "bb8")]
type Pool = bb8_redis::bb8::Pool<bb8_redis::RedisConnectionManager>;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
type Pool = deadpool_redis::Pool;

static POOL: OnceLock<Pool> = OnceLock::new();

pub fn pool() -> Pool {
    fn redis_url() -> String {
        if let Err(err) = dotenvy::dotenv() {
            warn!(?err, "Failed to initialize env variables");
        }

        tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter(EnvFilter::from_default_env())
            .compact()
            .init();

        env::var("REDIS_URL").unwrap_or_else(|_| {
            panic!(
                "Integration tests require env variable `REDIS_URL`. You can specify it through a \
                 .env file."
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

    // cannot flush db on startup due to potentially initializing multiple times
    // cannot flush db on cleanup due do lacking async drop
    POOL.get_or_init(init).clone()
}
