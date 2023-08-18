#![cfg(feature = "cold_resume")]

use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::BuildHasher,
    time::Duration,
};

use rkyv::{
    ser::{
        serializers::{AlignedSerializer, BufferScratch, CompositeSerializer},
        Serializer,
    },
    AlignedVec, Deserialize, Infallible,
};
use tracing::{info, instrument, trace};
use twilight_gateway::Session;

use crate::{
    key::RedisKey, redis::Cmd, rkyv_util::session::SessionsWrapper, CacheError, CacheResult,
    RedisCache,
};

impl<C> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub async fn freeze<S>(
        &self,
        sessions: &HashMap<u64, Session, S>,
        expire: Option<Duration>,
    ) -> CacheResult<()> {
        let wrapper = SessionsWrapper::new(sessions);

        // Using an `rkyv::ser::serializers::ScratchTracker`, checking `max_bytes_allocated`
        // turned out to be 24 when serializing a single session so we can just multiply
        // that to allocate only the minimum required scratch space.
        let mut scratch: Vec<u8> = vec![0; 24 * sessions.len()];

        let mut serializer = CompositeSerializer::new(
            AlignedSerializer::new(AlignedVec::new()),
            BufferScratch::new(&mut scratch),
            Infallible,
        );

        serializer
            .serialize_value(&wrapper)
            .map_err(CacheError::SerializeSessions)?;

        let bytes = serializer.into_serializer().into_inner();
        trace!(bytes = bytes.len());

        let mut conn = self.connection().await?;

        let cmd = match expire {
            Some(duration) => Cmd::set_ex(
                RedisKey::Sessions,
                bytes.as_slice(),
                duration.as_secs() as usize,
            ),
            None => Cmd::set(RedisKey::Sessions, bytes.as_slice()),
        };

        cmd.query_async(&mut conn).await?;

        Ok(())
    }

    #[instrument(level = "trace", name = "defrost", skip_all)]
    pub async fn defrost_with_hasher<S>(
        &self,
        flush_if_missing: bool,
    ) -> CacheResult<Option<HashMap<u64, Session, S>>>
    where
        S: BuildHasher + Default,
    {
        let mut conn = self.connection().await?;

        let bytes: Vec<u8> = Cmd::get(RedisKey::Sessions).query_async(&mut conn).await?;

        if bytes.is_empty() {
            if flush_if_missing {
                info!("Sessions not found; flushing redis database");

                Cmd::new().arg("FLUSHDB").query_async(&mut conn).await?;
            }

            return Ok(None);
        }

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<SessionsWrapper<S>>(&bytes)
            .map_err(|e| crate::CacheError::Validation(Box::new(e)))?;

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<SessionsWrapper<S>>(&bytes) };

        let sessions = archived.deserialize(&mut Infallible).unwrap();

        Ok(Some(sessions))
    }

    pub async fn defrost(
        &self,
        flush_if_missing: bool,
    ) -> CacheResult<Option<HashMap<u64, Session>>> {
        self.defrost_with_hasher::<RandomState>(flush_if_missing)
            .await
    }
}
