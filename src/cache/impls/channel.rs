use twilight_model::{
    channel::Channel,
    gateway::payload::incoming::ChannelPinsUpdate,
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
};

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, Cacheable, ICachedChannel},
    error::{SerializeError, UpdateError},
    key::RedisKey,
    util::{BytesArg, ZippedVecs},
    CacheResult, RedisCache,
};

type ChannelSerializer<'a, C> = <<C as CacheConfig>::Channel<'a> as Cacheable>::Serializer;

impl<C: CacheConfig> RedisCache<C> {
    pub(crate) fn store_channel(
        &self,
        pipe: &mut Pipe<'_, C>,
        channel: &Channel,
    ) -> CacheResult<()> {
        if C::Channel::WANTED {
            let guild_id = channel.guild_id;
            let channel_id = channel.id;
            let key = RedisKey::Channel { id: channel_id };
            let channel = C::Channel::from_channel(channel);

            let bytes = channel
                .serialize()
                .map_err(|e| SerializeError::Channel(Box::new(e)))?;

            pipe.set(key, bytes.as_ref(), C::Channel::expire_seconds())
                .ignore();

            if let Some(guild_id) = guild_id {
                let key = RedisKey::GuildChannels { id: guild_id };
                pipe.sadd(key, channel_id.get()).ignore();
            }

            let key = RedisKey::Channels;
            pipe.sadd(key, channel_id.get()).ignore();
        }

        if let Some(ref member) = channel.member {
            if let (Some(guild_id), Some(member)) = (channel.guild_id, &member.member) {
                self.store_member(pipe, guild_id, member)?;
            }

            if let Some(ref presence) = member.presence {
                self.store_presence(pipe, presence)?;
            }
        }

        if let Some(ref users) = channel.recipients {
            self.store_users(pipe, users)?;
        }

        Ok(())
    }

    pub(crate) async fn store_channel_pins_update(
        &self,
        pipe: &mut Pipe<'_, C>,
        update: &ChannelPinsUpdate,
    ) -> CacheResult<()> {
        if !C::Channel::WANTED {
            return Ok(());
        }

        let Some(update_fn) = C::Channel::on_pins_update() else {
            return Ok(());
        };

        let key = RedisKey::Channel {
            id: update.channel_id,
        };

        let Some(mut channel) = pipe.get::<C::Channel<'static>>(key).await? else {
            return Ok(());
        };

        update_fn(&mut channel, update).map_err(UpdateError::ChannelPins)?;

        let key = RedisKey::Channel {
            id: update.channel_id,
        };
        let bytes = channel.into_bytes();
        pipe.set(key, &bytes, C::Channel::expire_seconds()).ignore();

        Ok(())
    }

    pub(crate) fn store_channels(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
        channels: &[Channel],
    ) -> CacheResult<()> {
        if C::Channel::WANTED {
            let mut serializer = ChannelSerializer::<C>::default();

            let (channels, channel_ids) = channels
                .iter()
                .map(|channel| {
                    let id = channel.id;
                    let key = RedisKey::Channel { id };
                    let channel = C::Channel::from_channel(channel);

                    let bytes = channel
                        .serialize_with(&mut serializer)
                        .map_err(|e| SerializeError::Channel(Box::new(e)))?;

                    Ok(((key, BytesArg(bytes)), id.get()))
                })
                .collect::<CacheResult<ZippedVecs<(RedisKey, BytesArg), u64>>>()?
                .unzip();

            if !channels.is_empty() {
                pipe.mset(&channels, C::Channel::expire_seconds()).ignore();

                let key = RedisKey::GuildChannels { id: guild_id };
                pipe.sadd(key, channel_ids.as_slice()).ignore();

                let key = RedisKey::Channels;
                pipe.sadd(key, channel_ids).ignore();
            }
        }

        let users = channels
            .iter()
            .filter_map(|channel| channel.recipients.as_ref())
            .flatten();

        self.store_users(pipe, users)?;

        Ok(())
    }

    pub(crate) fn delete_channel(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Option<Id<GuildMarker>>,
        channel_id: Id<ChannelMarker>,
    ) {
        if !C::Channel::WANTED {
            return;
        }

        let key = RedisKey::Channel { id: channel_id };
        pipe.del(key).ignore();

        if let Some(guild_id) = guild_id {
            let key = RedisKey::GuildChannels { id: guild_id };
            pipe.srem(key, channel_id.get()).ignore();
        }

        let key = RedisKey::Channels;
        pipe.srem(key, channel_id.get()).ignore();
    }
}
