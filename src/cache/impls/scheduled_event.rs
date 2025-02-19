use bb8_redis::redis::Pipeline;
use rkyv::{api::high::to_bytes_in, rancor::Source, ser::writer::Buffer, Archived};
use tracing::{instrument, trace};
use twilight_model::{
    gateway::payload::incoming::{GuildScheduledEventUserAdd, GuildScheduledEventUserRemove},
    guild::scheduled_event::GuildScheduledEvent,
    id::{
        marker::{GuildMarker, ScheduledEventMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, HasArchived, IMeta, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedScheduledEvent, SerializeMany},
    error::{
        MetaError, MetaErrorKind, SerializeError, SerializeErrorKind, UpdateError, UpdateErrorKind,
    },
    rkyv_util::id::IdRkyv,
    util::{BytesWrap, ZippedVecs},
    CacheResult, RedisCache, RedisKey,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_scheduled_event(
        &self,
        pipe: &mut Pipe<'_, C>,
        event: &GuildScheduledEvent,
    ) -> CacheResult<()> {
        if let Some(ref user) = event.creator {
            self.store_user(pipe, user)?;
        }

        if !C::ScheduledEvent::WANTED {
            return Ok(());
        }

        let event_id = event.id;
        let guild_id = event.guild_id;

        let key = RedisKey::ScheduledEvent { id: event_id };

        let event = C::ScheduledEvent::from_scheduled_event(event);

        let bytes = event
            .serialize_one()
            .map_err(|e| SerializeError::new(e, SerializeErrorKind::ScheduledEvent))?;

        trace!(bytes = bytes.as_ref().len());

        pipe.set(key, bytes.as_ref(), C::ScheduledEvent::expire());

        let key = RedisKey::GuildScheduledEvents { id: guild_id };
        pipe.sadd(key, event_id.get());

        let key = RedisKey::ScheduledEvents;
        pipe.sadd(key, event_id.get());

        if C::ScheduledEvent::expire().is_some() {
            let key = ScheduledEventMetaKey { event: event_id };

            ScheduledEventMeta { guild: guild_id }
                .store(pipe, key)
                .map_err(|e| MetaError::new(e, MetaErrorKind::ScheduledEvent))?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn store_scheduled_events(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        events: &[GuildScheduledEvent],
    ) -> CacheResult<()> {
        if !C::ScheduledEvent::WANTED {
            return Ok(());
        }

        let mut serializer = C::ScheduledEvent::serialize_many();

        let (event_entries, event_ids): (Vec<_>, Vec<_>) = events
            .iter()
            .map(|stage_instance| {
                let id = stage_instance.id;
                let key = RedisKey::ScheduledEvent { id };
                let stage_instance = C::ScheduledEvent::from_scheduled_event(stage_instance);

                let bytes = serializer
                    .serialize_next(&stage_instance)
                    .map_err(|e| SerializeError::new(e, SerializeErrorKind::ScheduledEvent))?;

                trace!(bytes = bytes.as_ref().len());

                Ok(((key, BytesWrap(bytes)), id.get()))
            })
            .collect::<CacheResult<ZippedVecs<(RedisKey, BytesWrap<_>), u64>>>()?
            .unzip();

        if event_entries.is_empty() {
            return Ok(());
        }

        pipe.mset(&event_entries, C::ScheduledEvent::expire());

        let key = RedisKey::GuildScheduledEvents { id: guild_id };
        pipe.sadd(key, event_ids.as_slice());

        let key = RedisKey::ScheduledEvents;
        pipe.sadd(key, event_ids);

        if C::ScheduledEvent::expire().is_some() {
            events
                .iter()
                .try_for_each(|event| {
                    let key = ScheduledEventMetaKey { event: event.id };

                    ScheduledEventMeta { guild: guild_id }.store(pipe, key)
                })
                .map_err(|e| MetaError::new(e, MetaErrorKind::ScheduledEvent))?;
        }

        Ok(())
    }

    pub(crate) async fn store_scheduled_event_user_add(
        &self,
        pipe: &mut Pipe<'_, C>,
        event: &GuildScheduledEventUserAdd,
    ) -> CacheResult<()> {
        if !C::ScheduledEvent::WANTED {
            return Ok(());
        }

        let Some(update_fn) = C::ScheduledEvent::on_user_add_event() else {
            return Ok(());
        };

        let event_id = event.guild_scheduled_event_id;

        let key = RedisKey::ScheduledEvent { id: event_id };

        let Some(mut archived) = pipe.get::<C::ScheduledEvent<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut archived, event)
            .map_err(|e| UpdateError::new(e, UpdateErrorKind::ScheduledEventUserAdd))?;

        let key = RedisKey::ScheduledEvent { id: event_id };
        let bytes = archived.into_bytes();
        trace!(bytes = bytes.as_ref().len());
        pipe.set(key, &bytes, C::ScheduledEvent::expire());

        let key = RedisKey::ScheduledEvents;
        pipe.sadd(key, event_id.get());

        if C::ScheduledEvent::expire().is_some() {
            let meta = ScheduledEventMeta {
                guild: event.guild_id,
            };

            meta.store(pipe, ScheduledEventMetaKey { event: event_id })
                .map_err(|e| MetaError::new(e, MetaErrorKind::ScheduledEvent))?;
        }

        Ok(())
    }

    pub(crate) async fn store_scheduled_event_user_remove(
        &self,
        pipe: &mut Pipe<'_, C>,
        event: &GuildScheduledEventUserRemove,
    ) -> CacheResult<()> {
        if !C::ScheduledEvent::WANTED {
            return Ok(());
        }

        let Some(update_fn) = C::ScheduledEvent::on_user_remove_event() else {
            return Ok(());
        };

        let event_id = event.guild_scheduled_event_id;

        let key = RedisKey::ScheduledEvent { id: event_id };

        let Some(mut archived) = pipe.get::<C::ScheduledEvent<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut archived, event)
            .map_err(|e| UpdateError::new(e, UpdateErrorKind::ScheduledEventUserAdd))?;

        let key = RedisKey::ScheduledEvent { id: event_id };
        let bytes = archived.into_bytes();
        trace!(bytes = bytes.as_ref().len());
        pipe.set(key, &bytes, C::ScheduledEvent::expire());

        let key = RedisKey::ScheduledEvents;
        pipe.sadd(key, event_id.get());

        if C::ScheduledEvent::expire().is_some() {
            let meta = ScheduledEventMeta {
                guild: event.guild_id,
            };

            meta.store(pipe, ScheduledEventMetaKey { event: event_id })
                .map_err(|e| MetaError::new(e, MetaErrorKind::ScheduledEvent))?;
        }

        Ok(())
    }

    pub(crate) fn delete_scheduled_event(
        &self,
        pipe: &mut Pipe<'_, C>,
        event: &GuildScheduledEvent,
    ) -> CacheResult<()> {
        if let Some(ref user) = event.creator {
            self.store_user(pipe, user)?;
        }

        if !C::ScheduledEvent::WANTED {
            return Ok(());
        }

        let event_id = event.id;
        let guild_id = event.guild_id;

        let key = RedisKey::ScheduledEvent { id: event_id };
        pipe.del(key);

        let key = RedisKey::GuildScheduledEvents { id: guild_id };
        pipe.srem(key, event_id.get());

        let key = RedisKey::ScheduledEvents;
        pipe.srem(key, event_id.get());

        if C::ScheduledEvent::expire().is_some() {
            let key = RedisKey::ScheduledEventMeta { id: event_id };
            pipe.del(key);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ScheduledEventMetaKey {
    event: Id<ScheduledEventMarker>,
}

impl IMetaKey for ScheduledEventMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|event| Self { event })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::ScheduledEvents;
        pipe.srem(key, self.event.get()).ignore();
    }
}

impl HasArchived for ScheduledEventMetaKey {
    type Meta = ScheduledEventMeta;

    fn redis_key(&self) -> RedisKey {
        RedisKey::ScheduledEventMeta { id: self.event }
    }

    fn handle_archived(&self, pipe: &mut Pipeline, archived: &Archived<Self::Meta>) {
        let key = RedisKey::GuildScheduledEvents {
            id: archived.guild.into(),
        };
        pipe.srem(key, self.event.get());
    }
}

#[derive(rkyv::Archive, rkyv::Serialize)]
pub(crate) struct ScheduledEventMeta {
    #[rkyv(with = IdRkyv)]
    guild: Id<GuildMarker>,
}

impl IMeta<ScheduledEventMetaKey> for ScheduledEventMeta {
    type Bytes = [u8; 8];

    fn to_bytes<E: Source>(&self) -> Result<Self::Bytes, E> {
        let mut bytes = [0; 8];
        to_bytes_in(self, Buffer::from(&mut bytes))?;

        Ok(bytes)
    }
}
