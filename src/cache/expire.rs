use std::{error::Error as StdError, fmt::Write};

use futures_util::StreamExt;
use tracing::{error, info, trace, warn};

use super::meta::MetaKey;
use crate::{
    config::{CacheConfig, Cacheable},
    error::ExpireError,
    redis::{aio::PubSub, Cmd, DedicatedConnection, Pipeline, Pool},
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    pub(super) async fn handle_expire(pool: &Pool) -> CacheResult<()> {
        let any_expire = C::Channel::expire().is_some()
            || C::Emoji::expire().is_some()
            || C::Guild::expire().is_some()
            || C::Integration::expire().is_some()
            || C::Member::expire().is_some()
            || C::Message::expire().is_some()
            || C::Presence::expire().is_some()
            || C::Role::expire().is_some()
            || C::ScheduledEvent::expire().is_some()
            || C::StageInstance::expire().is_some()
            || C::Sticker::expire().is_some()
            || C::User::expire().is_some()
            || C::VoiceState::expire().is_some();

        if !any_expire {
            return Ok(());
        }

        let mut conn = DedicatedConnection::get(pool)
            .await
            .map_err(ExpireError::GetConnection)?;

        prepare_setting(&mut conn).await?;

        let mut pubsub = conn.into_pubsub();

        pubsub
            .psubscribe("*")
            .await
            .map_err(ExpireError::Subscribe)?;

        let conn = DedicatedConnection::get(pool)
            .await
            .map_err(ExpireError::GetConnection)?;

        tokio::spawn(listen_to_expire(pubsub, conn));

        Ok(())
    }
}

/// See <https://redis.io/docs/manual/keyspace-notifications/>
async fn prepare_setting(conn: &mut DedicatedConnection) -> CacheResult<()> {
    const SETTING_NAME: &str = "notify-keyspace-events";
    const EVENT_FLAG: char = 'E';
    const EXPIRE_FLAG: char = 'x';

    let mut setting = Cmd::new()
        .arg("CONFIG")
        .arg("GET")
        .arg(SETTING_NAME)
        .query_async::<_, Vec<String>>(conn)
        .await
        .map_err(ExpireError::GetSetting)?
        .pop()
        .unwrap_or_default();

    trace!(value = setting, "Current {SETTING_NAME}");

    if setting.contains(EVENT_FLAG) && setting.contains(EXPIRE_FLAG) {
        return Ok(());
    }

    if !setting.contains(EVENT_FLAG) {
        setting.push(EVENT_FLAG);
    }

    if !setting.contains(EXPIRE_FLAG) {
        setting.push(EXPIRE_FLAG);
    }

    Cmd::new()
        .arg("CONFIG")
        .arg("SET")
        .arg(SETTING_NAME)
        .arg(setting.as_str())
        .query_async::<_, ()>(conn)
        .await
        .map_err(ExpireError::SetSetting)?;

    info!(
        value = setting,
        "Successfully modified {SETTING_NAME} to listen to expire events"
    );

    Ok(())
}

async fn listen_to_expire(pubsub: PubSub, mut conn: DedicatedConnection) {
    let mut msgs = pubsub.into_on_message();
    let mut pipe = Pipeline::new();

    trace!("Listening to expire events...");

    while let Some(msg) = msgs.next().await {
        let channel = msg.get_channel_name();

        if !(channel.starts_with("__keyevent") && channel.ends_with("__:expired")) {
            continue;
        }

        let key = msg.get_payload_bytes();

        if let Err(err) = handle_expire(&mut conn, &mut pipe, key).await {
            unwind_error(&err);
        }

        pipe.clear();
    }

    warn!("Stopped listening to expire events");
}

async fn handle_expire(
    conn: &mut DedicatedConnection,
    pipe: &mut Pipeline,
    key: &[u8],
) -> Result<(), ExpireError> {
    let mut split = key.split(|&byte| byte == b':');

    let Some(key) = MetaKey::parse(&mut split) else {
        return Ok(());
    };

    key.handle_expire(conn, pipe).await?;

    pipe.query_async::<_, ()>(conn)
        .await
        .map_err(ExpireError::Pipe)?;

    Ok(())
}

fn unwind_error(err: &ExpireError) {
    let mut buf = "Failed to handle expire event: ".to_owned();
    let _ = write!(buf, "{err}");
    let mut e: &dyn StdError = err;

    while let Some(source) = e.source() {
        let _ = write!(buf, ": {source}");
        e = source;
    }

    error!("{buf}");
}
