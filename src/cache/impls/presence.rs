use tracing::{instrument, trace};
use twilight_model::{
    gateway::presence::{Presence, UserOrId},
    id::{marker::GuildMarker, Id},
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedPresence},
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type PresenceSerializer<'a, C> = <<C as CacheConfig>::Presence<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_presence(
        &self,
        pipe: &mut Pipe<'_, C>,
        presence: &Presence,
    ) -> CacheResult<()> {
        if C::Presence::WANTED {
            let guild_id = presence.guild_id;
            let user_id = presence.user.id();
            let key = RedisKey::Presence {
                guild: guild_id,
                user: user_id,
            };
            let presence = C::Presence::from_presence(presence);

            let bytes = presence
                .serialize()
                .map_err(|e| SerializeError::Presence(Box::new(e)))?;

            trace!(bytes = bytes.len());

            pipe.set(key, bytes.as_ref(), C::Presence::expire_seconds())
                .ignore();

            let key = RedisKey::GuildPresences { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if let UserOrId::User(ref user) = presence.user {
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_presences(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        presences: &[Presence],
    ) -> CacheResult<()> {
        if C::Presence::WANTED {
            let mut serializer = PresenceSerializer::<C>::default();

            let (presences, user_ids) = presences
                .iter()
                .map(|presence| {
                    let guild_id = presence.guild_id;
                    let user_id = presence.user.id();
                    let key = RedisKey::Presence {
                        guild: guild_id,
                        user: user_id,
                    };
                    let presence = C::Presence::from_presence(presence);

                    let bytes = presence
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError::Presence(Box::new(e)))?;

                    trace!(bytes = bytes.len());

                    Ok(((key, BytesArg(bytes)), user_id.get()))
                })
                .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
                .unzip();

            if !presences.is_empty() {
                pipe.mset(&presences, C::Presence::expire_seconds())
                    .ignore();

                let key = RedisKey::GuildPresences { id: guild_id };
                pipe.sadd(key, user_ids.as_slice()).ignore();
            }
        }

        let users = presences.iter().filter_map(|presence| match presence.user {
            UserOrId::User(ref user) => Some(user),
            UserOrId::UserId { .. } => None,
        });

        self.store_users(pipe, users)?;

        Ok(())
    }
}
