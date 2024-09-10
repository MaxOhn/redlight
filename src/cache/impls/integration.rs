use tracing::{instrument, trace};
use twilight_model::{
    guild::GuildIntegration,
    id::{
        marker::{GuildMarker, IntegrationMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedIntegration},
    error::{SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
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
                .serialize_one()
                .map_err(|e| SerializeError::new(e, SerializeErrorKind::Integration))?;

            trace!(bytes = bytes.as_ref().len());

            pipe.set(key, bytes.as_ref(), C::Integration::expire());

            let key = RedisKey::GuildIntegrations { id: guild_id };
            pipe.sadd(key, integration_id.get());
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
        pipe.del(key);

        let key = RedisKey::GuildIntegrations { id: guild_id };
        pipe.srem(key, integration_id.get());
    }
}

#[derive(Debug)]
pub(crate) struct IntegrationMetaKey {
    guild: Id<GuildMarker>,
    integration: Id<IntegrationMarker>,
}

impl IMetaKey for IntegrationMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split
            .next()
            .and_then(atoi)
            .zip(split.next().and_then(atoi))
            .map(|(guild, integration)| Self { guild, integration })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::GuildIntegrations { id: self.guild };
        pipe.srem(key, self.integration.get());
    }
}
