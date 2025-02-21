use rkyv::Archived;
use tracing::{instrument, trace};
use twilight_model::{
    gateway::payload::incoming::invite_create::PartialUser,
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
    user::User,
};

use crate::{
    cache::{
        meta::{atoi, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedUser, SerializeMany},
    error::{SerializeError, SerializeErrorKind, UpdateError, UpdateErrorKind},
    key::RedisKey,
    redis::Pipeline,
    util::BytesWrap,
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_user(&self, pipe: &mut Pipe<'_, C>, user: &User) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        let id = user.id;
        let key = RedisKey::User { id };
        let user = C::User::from_user(user);

        let bytes = user
            .serialize_one()
            .map_err(|e| SerializeError::new(e, SerializeErrorKind::User))?;

        trace!(bytes = bytes.as_ref().len());

        pipe.set(key, bytes.as_ref(), C::User::expire());

        let key = RedisKey::Users;
        pipe.sadd(key, id.get());

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_users<'a, I>(&self, pipe: &mut Pipe<'_, C>, users: I) -> CacheResult<()>
    where
        I: IntoIterator<Item = &'a User>,
    {
        if !C::User::WANTED {
            return Ok(());
        }

        let mut serializer = C::User::serialize_many();

        let (users, user_ids) = users
            .into_iter()
            .map(|user| {
                let id = user.id;
                let key = RedisKey::User { id };
                let user = C::User::from_user(user);

                let bytes = serializer
                    .serialize_next(&user)
                    .map_err(|e| SerializeError::new(e, SerializeErrorKind::User))?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesWrap(bytes)), id.get()))
            })
            .collect::<CacheResult<(Vec<(RedisKey, BytesWrap<_>)>, Vec<u64>)>>()?;

        if users.is_empty() {
            return Ok(());
        }

        pipe.mset(&users, C::User::expire());

        let key = RedisKey::Users;
        pipe.sadd(key, user_ids);

        Ok(())
    }

    pub(crate) async fn store_partial_user(
        &self,
        pipe: &mut Pipe<'_, C>,
        partial_user: &PartialUser,
    ) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        let id = partial_user.id;

        let key = RedisKey::Users;
        pipe.sadd(key, id.get());

        let Some(update_fn) = C::User::update_via_partial() else {
            return Ok(());
        };

        let key = RedisKey::User { id };

        let Some(mut user) = pipe.get::<Archived<C::User<'static>>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut user, partial_user)
            .map_err(|e| UpdateError::new(e, UpdateErrorKind::PartialUser))?;

        let key = RedisKey::User { id };
        let bytes = user.into_bytes();
        pipe.set(key, &bytes, C::Guild::expire());

        Ok(())
    }

    pub(crate) async fn delete_user(
        &self,
        pipe: &mut Pipe<'_, C>,
        user_id: Id<UserMarker>,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        if !C::User::WANTED {
            return Ok(());
        }

        debug_assert!(pipe.is_empty());

        let key = RedisKey::UserGuilds { id: user_id };
        pipe.srem(key, guild_id.get());

        let key = RedisKey::UserGuilds { id: user_id };
        pipe.scard(key);

        let common_guild_count: usize = pipe.query().await?;

        if common_guild_count == 0 {
            let key = RedisKey::User { id: user_id };
            pipe.del(key);

            let key = RedisKey::Users;
            pipe.srem(key, user_id.get());
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct UserMetaKey {
    user: Id<UserMarker>,
}

impl IMetaKey for UserMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|user| Self { user })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::Users;
        pipe.srem(key, self.user.get()).ignore();

        let key = RedisKey::UserGuilds { id: self.user };
        pipe.del(key).ignore();
    }
}

impl UserMetaKey {
    pub(crate) const fn new(user: Id<UserMarker>) -> Self {
        Self { user }
    }
}
