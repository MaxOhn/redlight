use twilight_model::{
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
    voice::VoiceState,
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedVoiceState},
    error::SerializeError,
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheError, CacheResult, RedisCache,
};

type VoiceStateSerializer<'a, C> = <<C as CacheConfig>::VoiceState<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    pub(crate) fn store_voice_state(
        &self,
        pipe: &mut Pipe<'_, C>,
        channel_id: Id<ChannelMarker>,
        voice_state: &VoiceState,
    ) -> CacheResult<()> {
        let Some(guild_id) = voice_state.guild_id else {
            return Ok(());
        };

        if C::VoiceState::WANTED {
            let user_id = voice_state.user_id;
            let key = RedisKey::VoiceState {
                guild: guild_id,
                user: user_id,
            };
            let voice_state = C::VoiceState::from_voice_state(channel_id, guild_id, voice_state);

            let bytes = voice_state
                .serialize()
                .map_err(|e| SerializeError::VoiceState(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::VoiceState::expire_seconds())
                .ignore();

            let key = RedisKey::GuildVoiceStates { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if let Some(ref member) = voice_state.member {
            self.store_member(pipe, guild_id, member)?;
        }

        Ok(())
    }

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
                    .map(|bytes| ((key, BytesArg(bytes)), user_id.get()))
                    .map_err(|e| {
                        CacheError::Serialization(SerializeError::VoiceState(Box::new(e)))
                    });

                Some(res)
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
            .unzip();

        if voice_states.is_empty() {
            return Ok(());
        }

        pipe.mset(&voice_states, C::VoiceState::expire_seconds())
            .ignore();

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.sadd(key, user_ids.as_slice()).ignore();

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
        pipe.del(key).ignore();

        let key = RedisKey::GuildVoiceStates { id: guild_id };
        pipe.srem(key, user_id.get()).ignore();
    }
}