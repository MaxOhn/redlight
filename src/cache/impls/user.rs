use tracing::{instrument, trace};
use twilight_model::{gateway::payload::incoming::invite_create::PartialUser, user::User};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedUser},
    error::{SerializeError, UpdateError},
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type UserSerializer<'a, C> = <<C as CacheConfig>::User<'a> as Cacheable>::Serializer;

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
            .serialize()
            .map_err(|e| SerializeError::User(Box::new(e)))?;

        trace!(bytes = bytes.len());

        pipe.set(key, bytes.as_ref(), C::User::expire_seconds())
            .ignore();

        let key = RedisKey::Users;
        pipe.sadd(key, id.get()).ignore();

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

        let mut serializer = UserSerializer::<C>::default();

        let (users, user_ids) = users
            .into_iter()
            .map(|user| {
                let id = user.id;
                let key = RedisKey::User { id };
                let user = C::User::from_user(user);

                let bytes = user
                    .serialize_with(&mut serializer)
                    .map_err(|e| SerializeError::User(Box::new(e)))?;

                trace!(bytes = bytes.len());

                Ok(((key, BytesArg(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if users.is_empty() {
            return Ok(());
        }

        pipe.mset(&users, C::User::expire_seconds()).ignore();

        let key = RedisKey::Users;
        pipe.sadd(key, user_ids).ignore();

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
        pipe.sadd(key, id.get()).ignore();

        let Some(update_fn) = C::User::update_via_partial() else {
            return Ok(());
        };

        let key = RedisKey::User { id };

        let Some(mut user) = pipe.get::<C::User<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut user, partial_user).map_err(UpdateError::PartialUser)?;

        let key = RedisKey::User { id };
        let bytes = user.into_bytes();
        pipe.set(key, &bytes, C::Guild::expire_seconds()).ignore();

        Ok(())
    }
}
