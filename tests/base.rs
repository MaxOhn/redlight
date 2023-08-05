mod events;

use std::{env, sync::OnceLock};

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
