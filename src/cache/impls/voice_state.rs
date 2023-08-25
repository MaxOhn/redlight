use tracing::{instrument, trace};
use twilight_model::{
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
    voice::VoiceState,
};

use crate::{
    cache::{
        meta::{atoi, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedVoiceState},
    error::{SerializeError, SerializeErrorKind},
    key::RedisKey,
    redis::Pipeline,
    util::{BytesArg, ZippedVecs},
    CacheError, CacheResult, RedisCache,
};

type VoiceStateSerializer<'a, C> = <<C as CacheConfig>::VoiceState<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_voice_state(
        &self,
        pipe: &mut Pipe<'_, C>,
        channel_id: Id<ChannelMarker>,
        guild_id: Id<GuildMarker>,
        voice_state: &VoiceState,
    ) -> CacheResult<()> {
        if C::VoiceState::WANTED {
            let user_id = voice_state.user_id;
            let key = RedisKey::VoiceState {
                guild: guild_id,
                user: user_id,
            };
            let voice_state = C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);

            let bytes = voice_state.serialize().map_err(|e| SerializeError {
                error: Box::new(e),
                kind: SerializeErrorKind::VoiceState,
            })?;

            trace!(bytes = bytes.as_ref().len());

            pipe.set(key, bytes.as_ref(), C::VoiceState::expire());

            let key = RedisKey::GuildVoiceStates { id: guild_id };
            pipe.sadd(key, user_id.get());
        }

        if let Some(ref member) = voice_state.member {
            self.store_member(pipe, guild_id, member)?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_voice_states(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        voice_states: &[VoiceState],
    ) -> CacheResult<()> {
        if !C::VoiceState::WANTED {
            return Ok(());
        }

        let mut serializer = VoiceStateSerializer::<C>::default();

        let (voice_states, user_ids) = voice_states
            .iter()
            .filter_map(|voice_state| {
                let channel_id = voice_state.channel_id?;

                let user_id = voice_state.user_id;
                let key = RedisKey::VoiceState {
                    guild: guild_id,
                    user: user_id,
                };
                let voice_state =
                    C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);

                let res = voice_state
                    .serialize_with(&mut serializer)
                    .map(|bytes| {
                        trace!(bytes = bytes.as_ref().len());

                        ((key, BytesArg(bytes)), user_id.get())
                    })
                    .map_err(|e| {
                        CacheError::Serialization(SerializeError {
                            error: Box::new(e),
                            kind: SerializeErrorKind::VoiceState,
                        })
                    });

                Some(res)
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg<_>), u64>>>()?
            .unzip();

        if voice_states.is_empty() {
            return Ok(());
        }

        pipe.mset(&voice_states, C::VoiceState::expire());

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.sadd(key, user_ids.as_slice());

        Ok(())
    }

    pub(crate) fn delete_voice_state(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) {
        if !C::VoiceState::WANTED {
            return;
        }

        let key = RedisKey::VoiceState {
            guild: guild_id,
            user: user_id,
        };
        pipe.del(key);

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.srem(key, user_id.get());
    }
}

#[derive(Debug)]
pub(crate) struct VoiceStateMetaKey {
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
}

impl IMetaKey for VoiceStateMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split
            .next()
            .and_then(atoi)
            .zip(split.next().and_then(atoi))
            .map(|(guild, user)| VoiceStateMetaKey { guild, user })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::GuildVoiceStates { id: self.guild };
        pipe.srem(key, self.user.get());
    }
}
