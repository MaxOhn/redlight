use tracing::{instrument, trace};
use twilight_model::{
    gateway::payload::incoming::GuildUpdate,
    guild::Guild,
    id::{marker::GuildMarker, Id},
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedGuild},
    error::{SerializeError, UpdateError},
    key::RedisKey,
    CacheError, CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_guild(&self, pipe: &mut Pipe<'_, C>, guild: &Guild) -> CacheResult<()> {
        if C::Guild::WANTED {
            let guild_id = guild.id;
            let key = RedisKey::Guild { id: guild_id };
            let guild = C::Guild::from_guild(guild);

            let bytes = guild
                .serialize()
                .map_err(|e| SerializeError::Guild(Box::new(e)))?;

            trace!(bytes = bytes.len());

            pipe.set(key, bytes.as_ref(), C::Guild::expire_seconds())
                .ignore();

            let key = RedisKey::Guilds;
            pipe.sadd(key, guild_id.get()).ignore();

            let key = RedisKey::UnavailableGuilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        self.store_channels(pipe, guild.id, &guild.channels)?;
        self.store_emojis(pipe, guild.id, &guild.emojis)?;
        self.store_members(pipe, guild.id, &guild.members)?;
        self.store_presences(pipe, guild.id, &guild.presences)?;
        self.store_roles(pipe, guild.id, &guild.roles)?;
        self.store_stickers(pipe, guild.id, &guild.stickers)?;
        self.store_channels(pipe, guild.id, &guild.threads)?;
        self.store_stage_instances(pipe, guild.id, &guild.stage_instances)?;
        self.store_voice_states(pipe, guild.id, &guild.voice_states)?;

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn store_guild_update(
        &self,
        pipe: &mut Pipe<'_, C>,
        update: &GuildUpdate,
    ) -> CacheResult<()> {
        let guild_id = update.id;

        self.store_emojis(pipe, guild_id, &update.emojis)?;
        self.store_roles(pipe, guild_id, &update.roles)?;

        if !C::Guild::WANTED {
            return Ok(());
        }

        let key = RedisKey::Guilds;
        pipe.sadd(key, guild_id.get()).ignore();

        let key = RedisKey::UnavailableGuilds;
        pipe.srem(key, guild_id.get()).ignore();

        let Some(update_fn) = C::Guild::on_guild_update() else {
            return Ok(());
        };

        let key = RedisKey::Guild { id: guild_id };

        let Some(mut guild) = pipe.get::<C::Guild<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut guild, update).map_err(UpdateError::Guild)?;

        let key = RedisKey::Guild { id: guild_id };
        let bytes = guild.into_bytes();
        trace!(bytes = bytes.len());
        pipe.set(key, &bytes, C::Guild::expire_seconds()).ignore();

        Ok(())
    }

    pub(crate) async fn delete_guild(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        debug_assert!(pipe.is_empty());

        if C::Member::WANTED || C::User::WANTED {
            let key = RedisKey::GuildMembers { id: guild_id };
            pipe.smembers(key);
        }

        if C::Channel::WANTED {
            let key = RedisKey::GuildChannels { id: guild_id };
            pipe.smembers(key);
        }

        if C::Emoji::WANTED {
            let key = RedisKey::GuildEmojis { id: guild_id };
            pipe.smembers(key);
        }

        if C::Integration::WANTED {
            let key = RedisKey::GuildIntegrations { id: guild_id };
            pipe.smembers(key);
        }

        if C::Presence::WANTED {
            let key = RedisKey::GuildPresences { id: guild_id };
            pipe.smembers(key);
        }

        if C::Role::WANTED {
            let key = RedisKey::GuildRoles { id: guild_id };
            pipe.smembers(key);
        }

        if C::StageInstance::WANTED {
            let key = RedisKey::GuildStageInstances { id: guild_id };
            pipe.smembers(key);
        }

        if C::Sticker::WANTED {
            let key = RedisKey::GuildStickers { id: guild_id };
            pipe.smembers(key);
        }

        if C::VoiceState::WANTED {
            let key = RedisKey::GuildVoiceStates { id: guild_id };
            pipe.smembers(key);
        }

        if pipe.is_empty() {
            if C::Guild::WANTED {
                let key = RedisKey::Guild { id: guild_id };
                pipe.del(key).ignore();

                let key = RedisKey::Guilds;
                pipe.srem(key, guild_id.get()).ignore();
            }

            return Ok(());
        }

        let mut iter = pipe.query::<Vec<Vec<u64>>>().await?.into_iter();

        let mut keys_to_delete = Vec::new();

        if C::Member::WANTED || C::User::WANTED {
            let user_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            if C::User::WANTED {
                for &user_id in user_ids.iter() {
                    let user_id = Id::new(user_id);

                    let key = RedisKey::UserGuilds { id: user_id };
                    pipe.srem(key, guild_id.get()).ignore();

                    let key = RedisKey::UserGuilds { id: user_id };
                    pipe.scard(key);
                }

                let scards: Vec<usize> = pipe.query().await?;

                let user_keys = user_ids
                    .iter()
                    .zip(scards)
                    .filter(|(_, common_guild_count)| *common_guild_count == 0)
                    .map(|(user_id, _)| RedisKey::User {
                        id: Id::new(*user_id),
                    });

                keys_to_delete.extend(user_keys);

                let key = RedisKey::Users;
                pipe.srem(key, &user_ids).ignore();
            }

            if C::Member::WANTED {
                let key = RedisKey::GuildMembers { id: guild_id };
                keys_to_delete.push(key);

                let member_keys = user_ids.iter().map(|&user_id| RedisKey::Member {
                    guild: guild_id,
                    user: Id::new(user_id),
                });

                keys_to_delete.extend(member_keys);
            }
        }

        if C::Channel::WANTED {
            let key = RedisKey::GuildChannels { id: guild_id };
            keys_to_delete.push(key);

            let channel_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Channels;
            pipe.srem(key, channel_ids.as_slice()).ignore();

            let channel_keys = channel_ids.into_iter().map(|channel_id| RedisKey::Channel {
                id: Id::new(channel_id),
            });

            keys_to_delete.extend(channel_keys);
        }

        if C::Emoji::WANTED {
            let key = RedisKey::GuildEmojis { id: guild_id };
            keys_to_delete.push(key);

            let emoji_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Emojis;
            pipe.srem(key, emoji_ids.as_slice()).ignore();

            let emoji_keys = emoji_ids.into_iter().map(|emoji_id| RedisKey::Emoji {
                id: Id::new(emoji_id),
            });

            keys_to_delete.extend(emoji_keys);
        }

        if C::Integration::WANTED {
            let key = RedisKey::GuildIntegrations { id: guild_id };
            keys_to_delete.push(key);

            let integration_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let integration_keys =
                integration_ids
                    .into_iter()
                    .map(|integration_id| RedisKey::Integration {
                        guild: guild_id,
                        id: Id::new(integration_id),
                    });

            keys_to_delete.extend(integration_keys);
        }

        if C::Presence::WANTED {
            let key = RedisKey::GuildPresences { id: guild_id };
            keys_to_delete.push(key);

            let user_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let presence_keys = user_ids.into_iter().map(|user_id| RedisKey::Presence {
                guild: guild_id,
                user: Id::new(user_id),
            });

            keys_to_delete.extend(presence_keys);
        }

        if C::Role::WANTED {
            let key = RedisKey::GuildRoles { id: guild_id };
            keys_to_delete.push(key);

            let role_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Roles;
            pipe.srem(key, role_ids.as_slice()).ignore();

            let role_keys = role_ids.into_iter().map(|role_id| RedisKey::Role {
                id: Id::new(role_id),
            });

            keys_to_delete.extend(role_keys);
        }

        if C::StageInstance::WANTED {
            let key = RedisKey::GuildStageInstances { id: guild_id };
            keys_to_delete.push(key);

            let stage_instance_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::StageInstances;
            pipe.srem(key, stage_instance_ids.as_slice()).ignore();

            let stage_instance_keys =
                stage_instance_ids
                    .into_iter()
                    .map(|stage_instance_id| RedisKey::StageInstance {
                        id: Id::new(stage_instance_id),
                    });

            keys_to_delete.extend(stage_instance_keys);
        }

        if C::Sticker::WANTED {
            let key = RedisKey::GuildStickers { id: guild_id };
            keys_to_delete.push(key);

            let sticker_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let key = RedisKey::Stickers;
            pipe.srem(key, sticker_ids.as_slice()).ignore();

            let sticker_keys = sticker_ids.into_iter().map(|sticker_id| RedisKey::Sticker {
                id: Id::new(sticker_id),
            });

            keys_to_delete.extend(sticker_keys);
        }

        if C::VoiceState::WANTED {
            let key = RedisKey::GuildVoiceStates { id: guild_id };
            keys_to_delete.push(key);

            let user_ids = iter.next().ok_or(CacheError::InvalidResponse)?;

            let voice_state_keys = user_ids.into_iter().map(|user_id| RedisKey::VoiceState {
                guild: guild_id,
                user: Id::new(user_id),
            });

            keys_to_delete.extend(voice_state_keys);
        }

        if C::Guild::WANTED {
            let key = RedisKey::Guild { id: guild_id };
            keys_to_delete.push(key);

            let key = RedisKey::Guilds;
            pipe.srem(key, guild_id.get()).ignore();
        }

        if !keys_to_delete.is_empty() {
            pipe.del(keys_to_delete).ignore();
        }

        Ok(())
    }

    pub(crate) async fn delete_guilds(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_ids: &[u64],
    ) -> CacheResult<()> {
        debug_assert!(pipe.is_empty());

        let count = C::Channel::WANTED as usize
            + C::Emoji::WANTED as usize
            + C::Integration::WANTED as usize
            + (C::Member::WANTED || C::User::WANTED) as usize
            + C::Presence::WANTED as usize
            + C::Role::WANTED as usize
            + C::StageInstance::WANTED as usize
            + C::Sticker::WANTED as usize
            + C::VoiceState::WANTED as usize;

        if C::Member::WANTED || C::User::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildMembers {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Channel::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildChannels {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Emoji::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildEmojis {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Integration::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildIntegrations {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Presence::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildPresences {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Role::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildRoles {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::StageInstance::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildStageInstances {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::Sticker::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildStickers {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if C::VoiceState::WANTED {
            for &guild_id in guild_ids {
                let key = RedisKey::GuildVoiceStates {
                    id: Id::new(guild_id),
                };
                pipe.smembers(key);
            }
        }

        if pipe.is_empty() {
            if C::Guild::WANTED {
                let guild_keys: Vec<_> = guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::Guild {
                        id: Id::new(guild_id),
                    })
                    .collect();

                pipe.del(guild_keys).ignore();

                let key = RedisKey::Guilds;
                pipe.srem(key, guild_ids).ignore();
            }

            return Ok(());
        }

        let data = pipe.query::<Vec<Vec<u64>>>().await?;

        if data.len() != count * guild_ids.len() {
            return Err(CacheError::InvalidResponse);
        }

        let mut iter = data.into_iter();

        let mut keys_to_delete = Vec::new();

        if C::Member::WANTED || C::User::WANTED {
            let user_ids_unflattened = &iter.as_slice()[..guild_ids.len()];

            if C::User::WANTED {
                let user_ids: Vec<_> = user_ids_unflattened.iter().flatten().copied().collect();

                for (user_ids, guild_id) in user_ids_unflattened.iter().zip(guild_ids) {
                    for &user_id in user_ids {
                        let user_id = Id::new(user_id);

                        let key = RedisKey::UserGuilds { id: user_id };
                        pipe.srem(key, guild_id).ignore();

                        let key = RedisKey::UserGuilds { id: user_id };
                        pipe.scard(key);
                    }
                }

                let scards: Vec<usize> = pipe.query().await?;

                let key = RedisKey::Users;
                pipe.srem(key, &user_ids).ignore();

                let user_keys = user_ids
                    .iter()
                    .zip(scards)
                    .filter(|(_, common_guild_count)| *common_guild_count == 0)
                    .map(|(user_id, _)| RedisKey::User {
                        id: Id::new(*user_id),
                    });

                keys_to_delete.extend(user_keys);
            }

            if C::Member::WANTED {
                let guild_keys = guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::GuildMembers {
                        id: Id::new(guild_id),
                    });

                keys_to_delete.extend(guild_keys);

                let member_keys =
                    user_ids_unflattened
                        .iter()
                        .zip(guild_ids)
                        .flat_map(|(user_ids, guild_id)| {
                            user_ids.iter().map(|&user_id| RedisKey::Member {
                                guild: Id::new(*guild_id),
                                user: Id::new(user_id),
                            })
                        });

                keys_to_delete.extend(member_keys);
            }

            iter.by_ref().take(guild_ids.len()).for_each(|_| ());
        }

        if C::Channel::WANTED {
            let channel_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Channels;
            pipe.srem(key, channel_ids.as_slice()).ignore();

            let channel_keys = channel_ids.into_iter().map(|emoji_id| RedisKey::Channel {
                id: Id::new(emoji_id),
            });

            keys_to_delete.extend(channel_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildChannels {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Emoji::WANTED {
            let emoji_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Emojis;
            pipe.srem(key, emoji_ids.as_slice()).ignore();

            let emoji_keys = emoji_ids.into_iter().map(|emoji_id| RedisKey::Emoji {
                id: Id::new(emoji_id),
            });

            keys_to_delete.extend(emoji_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildEmojis {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Integration::WANTED {
            let integration_keys = iter.by_ref().take(guild_ids.len()).zip(guild_ids).flat_map(
                |(integration_ids, guild_id)| {
                    integration_ids
                        .into_iter()
                        .map(|integration_id| RedisKey::Integration {
                            guild: Id::new(*guild_id),
                            id: Id::new(integration_id),
                        })
                },
            );

            keys_to_delete.extend(integration_keys);

            let guild_keys =
                guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::GuildIntegrations {
                        id: Id::new(guild_id),
                    });

            keys_to_delete.extend(guild_keys);
        }

        if C::Presence::WANTED {
            let presence_keys = iter.by_ref().take(guild_ids.len()).zip(guild_ids).flat_map(
                |(user_ids, guild_id)| {
                    user_ids.into_iter().map(|user_id| RedisKey::Presence {
                        guild: Id::new(*guild_id),
                        user: Id::new(user_id),
                    })
                },
            );

            keys_to_delete.extend(presence_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildPresences {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Role::WANTED {
            let role_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Roles;
            pipe.srem(key, role_ids.as_slice()).ignore();

            let role_keys = role_ids.into_iter().map(|role_id| RedisKey::Role {
                id: Id::new(role_id),
            });

            keys_to_delete.extend(role_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildRoles {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::StageInstance::WANTED {
            let stage_instance_ids: Vec<_> =
                iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::StageInstances;
            pipe.srem(key, stage_instance_ids.as_slice()).ignore();

            let stage_instance_keys =
                stage_instance_ids
                    .into_iter()
                    .map(|stage_instance_id| RedisKey::StageInstance {
                        id: Id::new(stage_instance_id),
                    });

            keys_to_delete.extend(stage_instance_keys);

            let guild_keys =
                guild_ids
                    .iter()
                    .copied()
                    .map(|guild_id| RedisKey::GuildStageInstances {
                        id: Id::new(guild_id),
                    });

            keys_to_delete.extend(guild_keys);
        }

        if C::Sticker::WANTED {
            let sticker_ids: Vec<_> = iter.by_ref().take(guild_ids.len()).flatten().collect();

            let key = RedisKey::Stickers;
            pipe.srem(key, sticker_ids.as_slice()).ignore();

            let sticker_keys = sticker_ids.into_iter().map(|sticker_id| RedisKey::Sticker {
                id: Id::new(sticker_id),
            });

            keys_to_delete.extend(sticker_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildStickers {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::VoiceState::WANTED {
            let voice_state_keys = iter.by_ref().take(guild_ids.len()).zip(guild_ids).flat_map(
                |(user_ids, guild_id)| {
                    user_ids.into_iter().map(|user_id| RedisKey::VoiceState {
                        guild: Id::new(*guild_id),
                        user: Id::new(user_id),
                    })
                },
            );

            keys_to_delete.extend(voice_state_keys);

            let guild_keys = guild_ids
                .iter()
                .copied()
                .map(|guild_id| RedisKey::GuildVoiceStates {
                    id: Id::new(guild_id),
                });

            keys_to_delete.extend(guild_keys);
        }

        if C::Guild::WANTED {
            let guild_keys = guild_ids.iter().copied().map(|guild_id| RedisKey::Guild {
                id: Id::new(guild_id),
            });

            keys_to_delete.extend(guild_keys);

            let key = RedisKey::Guilds;
            pipe.srem(key, guild_ids).ignore();
        }

        if !keys_to_delete.is_empty() {
            pipe.del(keys_to_delete).ignore();
        }

        Ok(())
    }
}
