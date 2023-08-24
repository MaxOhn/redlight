use rkyv::{ser::serializers::BufferSerializer, AlignedBytes, Archived};
use tracing::{instrument, trace};
use twilight_model::{
    channel::StageInstance,
    id::{
        marker::{GuildMarker, StageMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, HasArchived, IMeta, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedStageInstance},
    error::{MetaError, MetaErrorKind, SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    rkyv_util::id::IdRkyv,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type StageInstanceSerializer<'a, C> =
    <<C as CacheConfig>::StageInstance<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_stage_instance(
        &self,
        pipe: &mut Pipe<'_, C>,
        stage_instance: &StageInstance,
    ) -> CacheResult<()> {
        if !C::StageInstance::WANTED {
            return Ok(());
        }

        let stage_instance_id = stage_instance.id;
        let guild_id = stage_instance.guild_id;
        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        let stage_instance = C::StageInstance::from_stage_instance(stage_instance);

        let bytes = stage_instance.serialize().map_err(|e| SerializeError {
            error: Box::new(e),
            kind: SerializeErrorKind::StageInstance,
        })?;

        trace!(bytes = bytes.as_ref().len());

        pipe.set(key, bytes.as_ref(), C::StageInstance::expire());

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_id.get());

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_id.get());

        if C::StageInstance::expire().is_some() {
            let key = StageInstanceMetaKey {
                stage: stage_instance_id,
            };

            StageInstanceMeta { guild: guild_id }
                .store(pipe, key)
                .map_err(|error| MetaError {
                    error,
                    kind: MetaErrorKind::StageInstance,
                })?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_stage_instances(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        stage_instances: &[StageInstance],
    ) -> CacheResult<()> {
        if !C::StageInstance::WANTED {
            return Ok(());
        }

        let mut serializer = StageInstanceSerializer::<C>::default();

        let (stage_instance_entries, stage_instance_ids): (Vec<_>, Vec<_>) = stage_instances
            .iter()
            .map(|stage_instance| {
                let id = stage_instance.id;
                let key = RedisKey::StageInstance { id };
                let stage_instance = C::StageInstance::from_stage_instance(stage_instance);

                let bytes = stage_instance
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError {
                        error: Box::new(e),
                        kind: SerializeErrorKind::StageInstance,
                    })?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg<_>), u64>>>()?
            .unzip();

        if stage_instance_entries.is_empty() {
            return Ok(());
        }

        pipe.mset(&stage_instance_entries, C::StageInstance::expire());

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_ids.as_slice());

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_ids);

        if C::StageInstance::expire().is_some() {
            stage_instances
                .iter()
                .try_for_each(|stage_instance| {
                    let key = StageInstanceMetaKey {
                        stage: stage_instance.id,
                    };

                    StageInstanceMeta { guild: guild_id }.store(pipe, key)
                })
                .map_err(|error| MetaError {
                    error,
                    kind: MetaErrorKind::StageInstance,
                })?;
        }

        Ok(())
    }

    pub(crate) fn delete_stage_instance(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        stage_instance_id: Id<StageMarker>,
    ) {
        if !C::StageInstance::WANTED {
            return;
        }

        let key = RedisKey::StageInstance {
            id: stage_instance_id,
        };
        pipe.del(key);

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.srem(key, stage_instance_id.get());

        let key = RedisKey::StageInstances;
        pipe.srem(key, stage_instance_id.get());

        if C::StageInstance::expire().is_some() {
            let key = RedisKey::StageInstanceMeta {
                id: stage_instance_id,
            };
            pipe.del(key);
        }
    }
}

#[derive(Debug)]
pub(crate) struct StageInstanceMetaKey {
    stage: Id<StageMarker>,
}

impl IMetaKey for StageInstanceMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|stage| Self { stage })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::StageInstances;
        pipe.srem(key, self.stage.get()).ignore();
    }
}

impl HasArchived for StageInstanceMetaKey {
    type Meta = StageInstanceMeta;

    fn redis_key(&self) -> RedisKey {
        RedisKey::StageInstanceMeta { id: self.stage }
    }

    fn handle_archived(&self, pipe: &mut Pipeline, archived: &Archived<Self::Meta>) {
        let key = RedisKey::GuildStageInstances {
            id: archived.guild.into(),
        };
        pipe.srem(key, self.stage.get());
    }
}

#[derive(rkyv::Archive, rkyv::Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub(crate) struct StageInstanceMeta {
    #[with(IdRkyv)]
    guild: Id<GuildMarker>,
}

impl IMeta<StageInstanceMetaKey> for StageInstanceMeta {
    type Serializer = BufferSerializer<AlignedBytes<8>>;
}
