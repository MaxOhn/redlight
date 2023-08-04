use twilight_model::{
    guild::GuildIntegration,
    id::{
        marker::{GuildMarker, IntegrationMarker},
        Id,
    },
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedIntegration},
    error::SerializeError,
    key::RedisKey,
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    pub(crate) fn store_integration(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        integration: &GuildIntegration,
    ) -> CacheResult<()> {
        if C::Integration::WANTED {
            let integration_id = integration.id;
            let key = RedisKey::Integration {
                guild: guild_id,
                id: integration_id,
            };
            let integration = C::Integration::from_integration(integration);

            let bytes = integration
                .serialize()
                .map_err(|e| SerializeError::Integration(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Integration::expire_seconds())
                .ignore();

            let key = RedisKey::GuildIntegrations { id: guild_id };
            pipe.sadd(key, integration_id.get()).ignore();
        }

        if let Some(ref user) = integration.user {
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    pub(crate) fn delete_integration(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        integration_id: Id<IntegrationMarker>,
    ) {
        if !C::Integration::WANTED {
            return;
        }

        let key = RedisKey::Integration {
            guild: guild_id,
            id: integration_id,
        };
        pipe.del(key).ignore();

        let key = RedisKey::GuildIntegrations { id: guild_id };
        pipe.srem(key, integration_id.get()).ignore();
    }
}
