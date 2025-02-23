use std::{
    collections::{hash_map::RandomState, HashMap},
    convert::Infallible,
    hash::BuildHasher,
    time::Duration,
};

use rkyv::{
    rancor::{BoxedError, ResultExt},
    util::AlignedVec,
    with::With,
};
use tracing::{info, instrument, trace};
use twilight_gateway::Session;

use crate::{
    error::{CacheError, ValidationError},
    key::RedisKey,
    redis::Cmd,
    rkyv_util::session::{ArchivedSessions, SessionsRkyv},
    CacheResult, RedisCache,
};

#[cfg_attr(all(docsrs, not(doctest)), doc(cfg(feature = "cold_resume")))]
impl<C> RedisCache<C> {
    /// Given a map of shard ids and sessions, store those sessions in the cache
    /// and optionally add an expiration duration.
    ///
    /// The suggested expire duration is 3 minutes. Longer durations would
    /// likely cause the gateway to invalidate the sessions and instruct a
    /// reconnect.
    ///
    /// To retrieve the stored sessions, use [`defrost`](RedisCache::defrost).
    #[instrument(level = "trace", skip_all)]
    pub async fn freeze<S>(
        &self,
        sessions: &HashMap<u32, Session, S>,
        expire: Option<Duration>,
    ) -> CacheResult<()>
    where
        S: Default + BuildHasher,
        S::Hasher: Default,
    {
        let sessions = With::<_, SessionsRkyv>::cast(sessions);

        let bytes = rkyv::api::high::to_bytes_in(sessions, AlignedVec::<8>::new())
            .map_err(CacheError::SerializeSessions)?;

        trace!(bytes = bytes.len());

        let mut conn = self.connection().await?;

        #[allow(clippy::cast_possible_truncation)]
        let cmd = match expire {
            Some(duration) => Cmd::set_ex(
                RedisKey::Sessions,
                bytes.as_slice(),
                duration.as_secs() as usize,
            ),
            None => Cmd::set(RedisKey::Sessions, bytes.as_slice()),
        };

        let _: () = cmd.query_async(&mut conn).await?;

        Ok(())
    }

    /// Retrieve stored sessions and provide them in a [`HashMap`] with the
    /// given hasher.
    ///
    /// If `flush_if_missing` is set to `true` and there are no stored sessions,
    /// the redis command `FLUSHDB` will be executed, clearing **all** data from
    /// the database and ensuring that no invalid cached data remains.
    ///
    /// To store sessions, use [`freeze`](RedisCache::freeze).
    #[instrument(level = "trace", name = "defrost", skip_all)]
    pub async fn defrost_with_hasher<S>(
        &self,
        flush_if_missing: bool,
    ) -> CacheResult<Option<HashMap<u32, Session, S>>>
    where
        S: BuildHasher + Default,
    {
        let mut conn = self.connection().await?;

        let bytes: Vec<u8> = Cmd::get(RedisKey::Sessions).query_async(&mut conn).await?;

        if bytes.is_empty() {
            if flush_if_missing {
                info!("Sessions not found; flushing redis database");

                let _: () = Cmd::new().arg("FLUSHDB").query_async(&mut conn).await?;
            }

            return Ok(None);
        }

        #[cfg(feature = "bytecheck")]
        let archived: &ArchivedSessions =
            rkyv::access::<_, BoxedError>(&bytes).map_err(ValidationError::from)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &ArchivedSessions = unsafe { rkyv::access_unchecked(&bytes) };

        let sessions = rkyv::api::deserialize_using::<_, _, Infallible>(
            With::<_, SessionsRkyv>::cast(archived),
            &mut (),
        );

        Ok(Some(sessions.always_ok()))
    }

    /// Retrieve stored sessions and provide them in a default [`HashMap`].
    ///
    /// If `flush_if_missing` is set to `true` and there are no stored sessions,
    /// the redis command `FLUSHDB` will be executed, clearing **all** data from
    /// the database and ensuring that no invalid cached data remains.
    ///
    /// To store sessions, use [`freeze`](RedisCache::freeze).
    pub async fn defrost(
        &self,
        flush_if_missing: bool,
    ) -> CacheResult<Option<HashMap<u32, Session>>> {
        self.defrost_with_hasher::<RandomState>(flush_if_missing)
            .await
    }
}
