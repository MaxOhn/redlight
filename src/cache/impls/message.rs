use tracing::{instrument, trace};
use twilight_model::{
    channel::Message,
    gateway::payload::incoming::MessageUpdate,
    id::{marker::MessageMarker, Id},
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedMessage, ReactionEvent},
    error::{SerializeError, UpdateError},
    key::RedisKey,
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
            let key = RedisKey::Message { id: msg.id };
            let msg = C::Message::from_message(msg);

            let bytes = msg
                .serialize()
                .map_err(|e| SerializeError::Message(Box::new(e)))?;

            trace!(bytes = bytes.len());

            pipe.set(key, bytes.as_ref(), C::Message::expire_seconds())
                .ignore();
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

        update_fn(&mut message, update).map_err(UpdateError::Message)?;

        let key = RedisKey::Message { id: update.id };
        let bytes = message.into_bytes();
        trace!(bytes = bytes.len());
        pipe.set(key, &bytes, C::Message::expire_seconds()).ignore();

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

        let id = event.message_id();
        let key = RedisKey::Message { id };

        let Some(mut message) = pipe.get::<C::Message<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut message, event).map_err(UpdateError::Reaction)?;

        let key = RedisKey::Message { id };
        let bytes = message.into_bytes();
        trace!(bytes = bytes.len());
        pipe.set(key, &bytes, C::Message::expire_seconds()).ignore();

        Ok(())
    }

    pub(crate) fn delete_message(&self, pipe: &mut Pipe<'_, C>, msg_id: Id<MessageMarker>) {
        if !C::Message::WANTED {
            return;
        }

        let key = RedisKey::Message { id: msg_id };
        pipe.del(key).ignore();
    }

    pub(crate) fn delete_messages(&self, pipe: &mut Pipe<'_, C>, msg_ids: &[Id<MessageMarker>]) {
        if !C::Message::WANTED || msg_ids.is_empty() {
            return;
        }

        let keys: Vec<_> = msg_ids
            .iter()
            .copied()
            .map(|id| RedisKey::Message { id })
            .collect();

        pipe.del(keys).ignore();
    }
}
