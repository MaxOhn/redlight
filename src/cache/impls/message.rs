use std::ptr;

use rkyv::{api::high::to_bytes_in, rancor::Source, ser::writer::Buffer};
use tracing::{instrument, trace};
use twilight_model::{
    channel::Message,
    gateway::payload::incoming::MessageUpdate,
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
};

use crate::{
    cache::{
        meta::{atoi, HasArchived, IMeta, IMetaKey},
        pipe::Pipe,
    },
    config::{CacheConfig, Cacheable, ICachedMessage, ReactionEvent},
    error::{
        MetaError, MetaErrorKind, SerializeError, SerializeErrorKind, UpdateError, UpdateErrorKind,
    },
    key::RedisKey,
    redis::Pipeline,
    rkyv_util::id::IdRkyv,
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn store_message(
        &self,
        pipe: &mut Pipe<'_, C>,
        msg: &Message,
    ) -> CacheResult<()> {
        if C::Message::WANTED {
            let msg_id = msg.id;
            let channel_id = msg.channel_id;
            let key = RedisKey::Message { id: msg_id };
            let score = -msg.timestamp.as_micros();
            let msg = C::Message::from_message(msg);

            let bytes = msg
                .serialize_one()
                .map_err(|e| SerializeError::new(e, SerializeErrorKind::Message))?;

            trace!(bytes = bytes.as_ref().len());

            pipe.set(key, bytes.as_ref(), C::Message::expire());

            let key = RedisKey::Messages;
            pipe.sadd(key, msg_id.get());

            let key = RedisKey::ChannelMessages {
                channel: channel_id,
            };
            pipe.zadd(key, msg_id.get(), score);

            if C::Message::expire().is_some() {
                let meta = MessageMeta {
                    channel: channel_id,
                };

                meta.store(pipe, MessageMetaKey { msg: msg_id })
                    .map_err(|e| MetaError::new(e, MetaErrorKind::Message))?;
            }
        }

        self.store_user(pipe, &msg.author)?;

        if let Some(guild_id) = msg.guild_id {
            if let Some(ref member) = msg.member {
                self.store_partial_member(pipe, guild_id, member).await?;
            }

            for mention in msg.mentions.iter() {
                if let Some(ref member) = mention.member {
                    self.store_partial_member(pipe, guild_id, member).await?;
                }
            }
        }

        if let Some(ref channel) = msg.thread {
            self.store_channel(pipe, channel)?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn store_message_update(
        &self,
        pipe: &mut Pipe<'_, C>,
        update: &MessageUpdate,
    ) -> CacheResult<()> {
        if let Some(ref user) = update.author {
            self.store_user(pipe, user)?;
        }

        if !C::Message::WANTED {
            return Ok(());
        }

        let Some(update_fn) = C::Message::on_message_update() else {
            return Ok(());
        };

        let key = RedisKey::Message { id: update.id };

        let Some(mut message) = pipe.get::<C::Message<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut message, update)
            .map_err(|e| UpdateError::new(e, UpdateErrorKind::Message))?;

        let key = RedisKey::Message { id: update.id };
        let bytes = message.into_bytes();
        trace!(bytes = bytes.as_ref().len());
        pipe.set(key, &bytes, C::Message::expire());

        let key = RedisKey::Messages;
        pipe.sadd(key, update.id.get());

        if C::Message::expire().is_some() {
            let meta = MessageMeta {
                channel: update.channel_id,
            };

            meta.store(pipe, MessageMetaKey { msg: update.id })
                .map_err(|e| MetaError::new(e, MetaErrorKind::Message))?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn handle_reaction(
        &self,
        pipe: &mut Pipe<'_, C>,
        event: ReactionEvent<'_>,
    ) -> CacheResult<()> {
        if !C::Message::WANTED {
            return Ok(());
        }

        let Some(update_fn) = C::Message::on_reaction_event() else {
            return Ok(());
        };

        let msg_id = event.message_id();
        let channel_id = event.channel_id();
        let key = RedisKey::Message { id: msg_id };

        let Some(mut message) = pipe.get::<C::Message<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut message, event)
            .map_err(|e| UpdateError::new(e, UpdateErrorKind::Reaction))?;

        let key = RedisKey::Message { id: msg_id };
        let bytes = message.into_bytes();
        trace!(bytes = bytes.as_ref().len());
        pipe.set(key, &bytes, C::Message::expire());

        let key = RedisKey::Messages;
        pipe.sadd(key, msg_id.get());

        if C::Message::expire().is_some() {
            let meta = MessageMeta {
                channel: channel_id,
            };

            meta.store(pipe, MessageMetaKey { msg: msg_id })
                .map_err(|e| MetaError::new(e, MetaErrorKind::Message))?;
        }

        Ok(())
    }

    pub(crate) fn delete_message(
        &self,
        pipe: &mut Pipe<'_, C>,
        msg_id: Id<MessageMarker>,
        channel_id: Id<ChannelMarker>,
    ) {
        if !C::Message::WANTED {
            return;
        }

        let key = RedisKey::Message { id: msg_id };
        pipe.del(key);

        let key = RedisKey::Messages;
        pipe.srem(key, msg_id.get());

        let key = RedisKey::ChannelMessages {
            channel: channel_id,
        };
        pipe.zrem(key, msg_id.get());

        if C::Message::expire().is_some() {
            pipe.del(RedisKey::MessageMeta { id: msg_id });
        }
    }

    pub(crate) fn delete_messages(
        &self,
        pipe: &mut Pipe<'_, C>,
        msg_ids: &[Id<MessageMarker>],
        channel_id: Id<ChannelMarker>,
    ) {
        if !C::Message::WANTED || msg_ids.is_empty() {
            return;
        }

        let keys: Vec<_> = if C::Message::expire().is_some() {
            msg_ids
                .iter()
                .copied()
                .flat_map(|id| [RedisKey::Message { id }, RedisKey::MessageMeta { id }])
                .collect()
        } else {
            msg_ids
                .iter()
                .copied()
                .map(|id| RedisKey::Message { id })
                .collect()
        };

        pipe.del(keys);

        #[allow(clippy::items_after_statements)]
        const fn ids_to_u64(msg_ids: &[Id<MessageMarker>]) -> &[u64] {
            let ptr = ptr::from_ref(msg_ids) as *const [u64];

            // SAFETY: Id<T> is a transparent wrapper of NonZeroU64
            // which is a transparent wrapper of u64
            unsafe { &*ptr }
        }

        let raw_msg_ids = ids_to_u64(msg_ids);

        let key = RedisKey::Messages;
        pipe.srem(key, raw_msg_ids);

        let key = RedisKey::ChannelMessages {
            channel: channel_id,
        };
        pipe.zrem(key, raw_msg_ids);
    }
}

#[derive(Debug)]
pub(crate) struct MessageMetaKey {
    msg: Id<MessageMarker>,
}

impl IMetaKey for MessageMetaKey {
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        split.next().and_then(atoi).map(|msg| Self { msg })
    }

    fn handle_expire(&self, pipe: &mut Pipeline) {
        let key = RedisKey::Messages;
        pipe.srem(key, self.msg.get()).ignore();
    }
}

impl HasArchived for MessageMetaKey {
    type Meta = MessageMeta;

    fn redis_key(&self) -> RedisKey {
        RedisKey::MessageMeta { id: self.msg }
    }

    fn handle_archived(&self, pipe: &mut Pipeline, archived: &rkyv::Archived<Self::Meta>) {
        let key = RedisKey::ChannelMessages {
            channel: archived.channel.into(),
        };
        pipe.zrem(key, self.msg.get()).ignore();
    }
}

#[derive(rkyv::Archive, rkyv::Serialize)]
pub(crate) struct MessageMeta {
    #[rkyv(with = IdRkyv)]
    channel: Id<ChannelMarker>,
}

impl IMeta<MessageMetaKey> for MessageMeta {
    type Bytes = [u8; 8];

    fn to_bytes<E: Source>(&self) -> Result<Self::Bytes, E> {
        let mut bytes = [0; 8];
        to_bytes_in(self, Buffer::from(&mut bytes))?;

        Ok(bytes)
    }
}
