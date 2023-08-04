use twilight_model::{
    channel::StageInstance,
    id::{
        marker::{GuildMarker, StageMarker},
        Id,
    },
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedStageInstance},
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type StageInstanceSerializer<'a, C> =
    <<C as CacheConfig>::StageInstance<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
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

        let bytes = stage_instance
            .serialize()
            .map_err(|e| SerializeError::StageInstance(Box::new(e)))?;

        pipe.set(key, bytes.as_ref(), C::StageInstance::expire_seconds())
            .ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_id.get()).ignore();

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_id.get()).ignore();

        Ok(())
    }

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

        let (stage_instances, stage_instance_ids): (Vec<_>, Vec<_>) = stage_instances
            .iter()
            .map(|stage_instance| {
                let id = stage_instance.id;
                let key = RedisKey::StageInstance { id };
                let stage_instance = C::StageInstance::from_stage_instance(stage_instance);
                let bytes = stage_instance
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::StageInstance(Box::new(e)))?;

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if stage_instances.is_empty() {
            return Ok(());
        }

        pipe.mset(&stage_instances, C::StageInstance::expire_seconds())
            .ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.sadd(key, stage_instance_ids.as_slice()).ignore();

        let key = RedisKey::StageInstances;
        pipe.sadd(key, stage_instance_ids).ignore();

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
        pipe.del(key).ignore();

        let key = RedisKey::GuildStageInstances { id: guild_id };
        pipe.srem(key, stage_instance_id.get()).ignore();

        let key = RedisKey::StageInstances;
        pipe.srem(key, stage_instance_id.get()).ignore();
    }
}
