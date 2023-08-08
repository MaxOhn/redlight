use twilight_model::{
    gateway::payload::incoming::MemberUpdate,
    guild::{Member, PartialMember},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedMember},
    error::{SerializeError, UpdateError},
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type MemberSerializer<'a, C> = <<C as CacheConfig>::Member<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    pub(crate) fn store_member(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        member: &Member,
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let user_id = member.user.id;
            let key = RedisKey::Member {
                guild: guild_id,
                user: user_id,
            };
            let member = C::Member::from_member(guild_id, member);

            let bytes = member
                .serialize()
                .map_err(|e| SerializeError::Member(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Member::expire_seconds())
                .ignore();

            let key = RedisKey::GuildMembers { id: guild_id };
            pipe.sadd(key, user_id.get()).ignore();
        }

        if C::User::WANTED {
            let key = RedisKey::UserGuilds { id: member.user.id };
            pipe.sadd(key, guild_id.get()).ignore();
        }

        self.store_user(pipe, &member.user)?;

        Ok(())
    }

    pub(crate) async fn store_member_update(
        &self,
        pipe: &mut Pipe<'_, C>,
        update: &MemberUpdate,
    ) -> CacheResult<()> {
        self.store_user(pipe, &update.user)?;

        let user_id = update.user.id;

        if C::User::WANTED {
            let key = RedisKey::UserGuilds { id: user_id };
            pipe.sadd(key, update.guild_id.get()).ignore();
        }

        if !C::Member::WANTED {
            return Ok(());
        }

        let key = RedisKey::GuildMembers {
            id: update.guild_id,
        };

        pipe.sadd(key, user_id.get()).ignore();

        let Some(update_fn) = C::Member::on_member_update() else {
            return Ok(());
        };

        let key = RedisKey::Member {
            guild: update.guild_id,
            user: user_id,
        };

        let Some(mut member) = pipe.get::<C::Member<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut member, update).map_err(UpdateError::Member)?;

        let key = RedisKey::Member {
            guild: update.guild_id,
            user: user_id,
        };

        let bytes = member.into_bytes();
        pipe.set(key, &bytes, C::Member::expire_seconds()).ignore();

        Ok(())
    }

    pub(crate) fn store_members(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        members: &[Member],
    ) -> CacheResult<()> {
        if C::Member::WANTED {
            let mut serializer = MemberSerializer::<C>::default();

            let (member_tuples, user_ids) = members
                .iter()
                .map(|member| {
                    let user_id = member.user.id;
                    let key = RedisKey::Member {
                        guild: guild_id,
                        user: user_id,
                    };
                    let member = C::Member::from_member(guild_id, member);

                    let bytes = member
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError::Member(Box::new(e)))?;

                    Ok(((key, BytesArg(bytes)), user_id.get()))
                })
                .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
                .unzip();

            if !member_tuples.is_empty() {
                pipe.mset(&member_tuples, C::Member::expire_seconds())
                    .ignore();

                let key = RedisKey::GuildMembers { id: guild_id };
                pipe.sadd(key, user_ids.as_slice()).ignore();

                if C::User::WANTED {
                    for member in members {
                        let key = RedisKey::UserGuilds { id: member.user.id };
                        pipe.sadd(key, guild_id.get()).ignore();
                    }
                }
            }
        } else if C::User::WANTED {
            for member in members {
                let key = RedisKey::UserGuilds { id: member.user.id };
                pipe.sadd(key, guild_id.get()).ignore();
            }
        }

        let users = members.iter().map(|member| &member.user);
        self.store_users(pipe, users)?;

        Ok(())
    }

    pub(crate) async fn store_partial_member(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        partial_member: &PartialMember,
    ) -> CacheResult<()> {
        if let Some(ref user) = partial_member.user {
            self.store_user(pipe, user)?;
        }

        let Some(ref user) = partial_member.user else {
            return Ok(());
        };

        if C::User::WANTED {
            let key = RedisKey::UserGuilds { id: user.id };
            pipe.sadd(key, guild_id.get()).ignore();
        }

        if !C::Member::WANTED {
            return Ok(());
        }

        let key = RedisKey::GuildMembers { id: guild_id };
        pipe.sadd(key, user.id.get()).ignore();

        let Some(update_fn) = C::Member::update_via_partial() else {
            return Ok(());
        };

        let key = RedisKey::Member {
            guild: guild_id,
            user: user.id,
        };

        let Some(mut member) = pipe.get::<C::Member<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut member, partial_member).map_err(UpdateError::PartialMember)?;

        let key = RedisKey::Member {
            guild: guild_id,
            user: user.id,
        };

        let bytes = member.into_bytes();
        pipe.set(key, &bytes, C::Member::expire_seconds()).ignore();

        Ok(())
    }

    pub(crate) async fn delete_member(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> CacheResult<()> {
        if C::User::WANTED {
            debug_assert!(pipe.is_empty());

            let key = RedisKey::UserGuilds { id: user_id };
            pipe.srem(key, guild_id.get()).ignore();

            let key = RedisKey::UserGuilds { id: user_id };
            pipe.scard(key);

            let common_guild_count: usize = pipe.query().await?;

            if common_guild_count == 0 {
                let key = RedisKey::User { id: user_id };
                pipe.del(key).ignore();

                let key = RedisKey::Users;
                pipe.srem(key, user_id.get()).ignore();
            }
        }

        if !C::Member::WANTED {
            return Ok(());
        }

        let key = RedisKey::Member {
            guild: guild_id,
            user: user_id,
        };
        pipe.del(key).ignore();

        let key = RedisKey::GuildMembers { id: guild_id };
        pipe.srem(key, user_id.get()).ignore();

        Ok(())
    }
}
