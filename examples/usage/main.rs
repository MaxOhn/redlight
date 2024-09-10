mod config;

use std::{env, error::Error};

use redlight::RedisCache;
use twilight_gateway::{Intents, Shard, ShardId};

use self::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize env variables from our .env file
    dotenvy::dotenv().unwrap();

    // Create our cache by using our custom `Config`
    let url = env::var("REDIS_URL").map_err(|e| format!("REDIS_URL env var error: {e}"))?;
    let cache = RedisCache::<Config>::new(&url).await?;

    // Create our gateway connection
    let token =
        env::var("DISCORD_TOKEN").map_err(|e| format!("DISCORD_TOKEN env var error: {e}"))?;
    let intents = Intents::GUILDS | Intents::GUILD_MEMBERS;
    let mut shard = Shard::new(ShardId::ONE, token, intents);

    // Receive events and update the cache
    loop {
        let event = shard.next_event().await.unwrap();
        cache.update(&event).await?;
    }
}
