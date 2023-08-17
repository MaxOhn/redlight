use tracing::{instrument, trace};
use twilight_model::user::CurrentUser;

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedCurrentUser},
    error::SerializeError,
    key::RedisKey,
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_current_user(
        &self,
        pipe: &mut Pipe<'_, C>,
        current_user: &CurrentUser,
    ) -> CacheResult<()> {
        if !C::CurrentUser::WANTED {
            return Ok(());
        }

        let key = RedisKey::CurrentUser;
        let current_user = C::CurrentUser::from_current_user(current_user);

        let bytes = current_user
            .serialize()
            .map_err(|e| SerializeError::CurrentUser(Box::new(e)))?;

        trace!(bytes = bytes.len());

        pipe.set(key, bytes.as_ref(), C::CurrentUser::expire_seconds())
            .ignore();

        Ok(())
    }
}
