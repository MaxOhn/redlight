use tracing::{instrument, trace};
use twilight_model::{
    guild::Role,
    id::{
        marker::{GuildMarker, RoleMarker},
        Id,
    },
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedRole},
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type RoleSerializer<'a, C> = <<C as CacheConfig>::Role<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_role(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        role: &Role,
    ) -> CacheResult<()> {
        if !C::Role::WANTED {
            return Ok(());
        }

        let id = role.id;
        let key = RedisKey::Role { id };
        let role = C::Role::from_role(role);

        let bytes = role
            .serialize()
            .map_err(|e| SerializeError::Role(Box::new(e)))?;

        trace!(bytes = bytes.len());

        pipe.set(key, bytes.as_ref(), C::Role::expire_seconds())
            .ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, id.get()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, id.get()).ignore();

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_roles<'a, I>(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        roles: I,
    ) -> CacheResult<()>
    where
        I: IntoIterator<Item = &'a Role>,
    {
        if !C::Role::WANTED {
            return Ok(());
        }

        let mut serializer = RoleSerializer::<C>::default();

        let (roles, role_ids) = roles
            .into_iter()
            .map(|role| {
                let id = role.id;
                let key = RedisKey::Role { id };
                let role = C::Role::from_role(role);

                let bytes = role
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::Role(Box::new(e)))?;

                trace!(bytes = bytes.len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if roles.is_empty() {
            return Ok(());
        }

        pipe.mset(&roles, C::Role::expire_seconds()).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, role_ids.as_slice()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, role_ids).ignore();

        Ok(())
    }

    pub(crate) fn delete_role(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        role_id: Id<RoleMarker>,
    ) {
        if !C::Role::WANTED {
            return;
        }

        let key = RedisKey::Role { id: role_id };
        pipe.del(key).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.srem(key, role_id.get()).ignore();

        let key = RedisKey::Roles;
        pipe.srem(key, role_id.get()).ignore();
    }
}
