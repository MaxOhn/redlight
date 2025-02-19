use metrics::{describe_gauge, gauge};
use tracing::{error, trace};

use super::RedisCache;
use crate::{
    config::{CacheConfig, Cacheable},
    key::RedisKey,
    redis::{Connection, Pipeline, Pool},
};

impl<C: CacheConfig> RedisCache<C> {
    pub(crate) fn init_metrics(pool: &Pool) {
        let wants_any = C::Channel::WANTED
            || C::Emoji::WANTED
            || C::Guild::WANTED
            || C::Message::WANTED
            || C::Role::WANTED
            || C::StageInstance::WANTED
            || C::Sticker::WANTED
            || C::User::WANTED;

        if !wants_any {
            return;
        }

        tokio::spawn(metrics_loop::<C>(pool.clone()));
    }
}

async fn metrics_loop<C: CacheConfig>(pool: Pool) {
    const CHANNEL_COUNT: &str = "channel_count";
    const EMOJI_COUNT: &str = "emoji_count";
    const GUILD_COUNT: &str = "guild_count";
    const MESSAGE_COUNT: &str = "message_count";
    const ROLE_COUNT: &str = "role_count";
    const SCHEDULED_EVENT_COUNT: &str = "scheduled_event_count";
    const STAGE_INSTANCE_COUNT: &str = "stage_instance_count";
    const STICKER_COUNT: &str = "sticker_count";
    const UNAVAILABLE_GUILD_COUNT: &str = "unavailable_guild_count";
    const USER_COUNT: &str = "user_count";

    describe_gauge!(CHANNEL_COUNT, "Amount of cached channels");
    describe_gauge!(EMOJI_COUNT, "Amount of cached emojis");
    describe_gauge!(GUILD_COUNT, "Amount of cached guilds");
    describe_gauge!(MESSAGE_COUNT, "Amount of cached messages");
    describe_gauge!(ROLE_COUNT, "Amount of cached roles");
    describe_gauge!(SCHEDULED_EVENT_COUNT, "Amount of cached scheduled events");
    describe_gauge!(STAGE_INSTANCE_COUNT, "Amount of cached stage instances");
    describe_gauge!(STICKER_COUNT, "Amount of cached stickers");
    describe_gauge!(UNAVAILABLE_GUILD_COUNT, "Amount of unavailable guilds");
    describe_gauge!(USER_COUNT, "Amount of cached users");

    let duration = C::METRICS_INTERVAL_DURATION;
    let mut pipe = Pipeline::new();
    let mut interval = tokio::time::interval(duration);

    trace!(interval = ?duration, "Running metrics loop");

    interval.tick().await;

    loop {
        interval.tick().await;

        add_scards::<C>(&mut pipe);

        let mut conn = match Connection::get(&pool).await {
            Ok(conn) => conn,
            Err(err) => {
                error!(%err, "Failed to acquire connection for metrics");

                continue;
            }
        };

        let mut scards = match pipe.query_async::<_, Vec<usize>>(&mut conn).await {
            Ok(scards) => scards.into_iter(),
            Err(err) => {
                error!(%err, "Failed to request metric values from redis");

                continue;
            }
        };

        pipe.clear();

        #[allow(clippy::cast_precision_loss)]
        let mut next_scard = || scards.next().unwrap_or(0) as f64;

        if C::Channel::WANTED {
            gauge!(CHANNEL_COUNT).set(next_scard());
        }

        if C::Emoji::WANTED {
            gauge!(EMOJI_COUNT).set(next_scard());
        }

        if C::Guild::WANTED {
            gauge!(GUILD_COUNT).set(next_scard());
            gauge!(UNAVAILABLE_GUILD_COUNT).set(next_scard());
        }

        if C::Message::WANTED {
            gauge!(MESSAGE_COUNT).set(next_scard());
        }

        if C::Role::WANTED {
            gauge!(ROLE_COUNT).set(next_scard());
        }

        if C::ScheduledEvent::WANTED {
            gauge!(SCHEDULED_EVENT_COUNT).set(next_scard());
        }

        if C::StageInstance::WANTED {
            gauge!(STAGE_INSTANCE_COUNT).set(next_scard());
        }

        if C::Sticker::WANTED {
            gauge!(STICKER_COUNT).set(next_scard());
        }

        if C::User::WANTED {
            gauge!(USER_COUNT).set(next_scard());
        }
    }
}

fn add_scards<C: CacheConfig>(pipe: &mut Pipeline) {
    if C::Channel::WANTED {
        pipe.scard(RedisKey::Channels);
    }

    if C::Emoji::WANTED {
        pipe.scard(RedisKey::Emojis);
    }

    if C::Guild::WANTED {
        pipe.scard(RedisKey::Guilds);
        pipe.scard(RedisKey::UnavailableGuilds);
    }

    if C::Message::WANTED {
        pipe.scard(RedisKey::Messages);
    }

    if C::Role::WANTED {
        pipe.scard(RedisKey::Roles);
    }

    if C::ScheduledEvent::WANTED {
        pipe.scard(RedisKey::ScheduledEvents);
    }

    if C::StageInstance::WANTED {
        pipe.scard(RedisKey::StageInstances);
    }

    if C::Sticker::WANTED {
        pipe.scard(RedisKey::Stickers);
    }

    if C::User::WANTED {
        pipe.scard(RedisKey::Users);
    }
}
