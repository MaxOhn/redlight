use rkyv::{ser::serializers::BufferSerializer, AlignedBytes, Archived};
use tracing::{instrument, trace};
use twilight_model::{
    guild::Role,
    id::{
        marker::{GuildMarker, RoleMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, HasArchived, IMeta, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedRole},
    error::{MetaError, MetaErrorKind, SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    rkyv_util::id::IdRkyv,
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

        let bytes = role.serialize().map_err(|e| SerializeError {
            error: Box::new(e),
            kind: SerializeErrorKind::Role,
        })?;

        trace!(bytes = bytes.as_ref().len());

        pipe.set(key, bytes.as_ref(), C::Role::expire()).ignore();

        let key = RedisKey::GuildRoles { id: guild_id };
        pipe.sadd(key, id.get()).ignore();

        let key = RedisKey::Roles;
        pipe.sadd(key, id.get()).ignore();

        if C::Role::expire().is_some() {
            RoleMeta { guild: guild_id }
                .store(pipe, RoleMetaKey { role: id })
                .map_err(|error| MetaError {
                    error,
                    kind: MetaErrorKind::Role,
                })?;
        }

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

        let with_expire = C::Role::expire().is_some();
        let mut serializer = RoleSerializer::<C>::default();

        let (roles, role_ids) = roles
            .into_iter()
            .map(|role| {
                let id = role.id;
                let key = RedisKey::Role { id };
                let role = C::Role::from_role(role);

                if with_expire {
                    RoleMeta { guild: guild_id }
                        .store(pipe, RoleMetaKey { role: id })
                        .map_err(|error| MetaError {
                            error,
                            kind: MetaErrorKind::Role,
                        })?;
                }

                let bytes = role
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError {
                        error: Box::new(e),
                        kind: SerializeErrorKind::Role,
                    })?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg<_>), u64>>>()?
            .unzip();

        if roles.is_empty() {
            return Ok(());
        }

        pipe.mset(&roles, C::Role::expire()).ignore();

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

        if C::Role::expire().is_some() {
            pipe.del(RedisKey::RoleMeta { id: role_id });
        }
    }
}

#[derive(Debug)]
pub(crate) struct RoleMetaKey {
    role: Id<RoleMarker>,
}

impl IMetaKey for RoleMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|role| Self { role })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::Roles;
        pipe.srem(key, self.role.get()).ignore();
    }
}

impl HasArchived for RoleMetaKey {
    type Meta = RoleMeta;

    fn redis_key(&self) -> RedisKey {
        RedisKey::RoleMeta { id: self.role }
    }

    fn handle_archived(&self, pipe: &mut Pipeline, archived: &Archived<Self::Meta>) {
        let key = RedisKey::GuildRoles {
            id: archived.guild.into(),
        };
        pipe.srem(key, self.role.get());
    }
}

#[derive(rkyv::Archive, rkyv::Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub(crate) struct RoleMeta {
    #[with(IdRkyv)]
    guild: Id<GuildMarker>,
}

impl IMeta<RoleMetaKey> for RoleMeta {
    type Serializer = BufferSerializer<AlignedBytes<8>>;
}
