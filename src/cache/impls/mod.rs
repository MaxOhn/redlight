mod channel;
mod current_user;
mod emoji;
mod guild;
mod integration;
mod member;
mod message;
mod presence;
mod role;
mod stage_instance;
mod sticker;
mod user;
mod voice_state;

use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    guild::UnavailableGuild,
    id::{marker::GuildMarker, Id},
};

use crate::{config::CacheConfig, key::RedisKey, CacheResult, RedisCache};

use super::pipe::Pipe;

impl<C: CacheConfig> RedisCache<C> {
    pub(crate) async fn store_interaction(
        &self,
        pipe: &mut Pipe<'_, C>,
        interaction: &Interaction,
    ) -> CacheResult<()> {
        if let Some(ref channel) = interaction.channel {
            self.store_channel(pipe, channel)?;
        }

        if let Some(InteractionData::ApplicationCommand(ref data)) = interaction.data {
            if let Some(ref resolved) = data.resolved {
                if let Some(guild_id) = interaction.guild_id {
                    let roles = resolved.roles.values();
                    self.store_roles(pipe, guild_id, roles)?;
                }

                let users = resolved.users.values();
                self.store_users(pipe, users)?;
            }
        }

        if let (Some(guild_id), Some(member)) = (interaction.guild_id, &interaction.member) {
            self.store_partial_member(pipe, guild_id, member).await?;
        }

        if let Some(ref msg) = interaction.message {
            self.store_message(pipe, msg).await?;
        }

        if let Some(ref user) = interaction.user {
            self.store_user(pipe, user)?;
        }

        Ok(())
    }

    pub(crate) async fn store_unavailable_guild(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        self.delete_guild(pipe, guild_id).await?;

        let key = RedisKey::UnavailableGuilds;
        pipe.sadd(key, guild_id.get()).ignore();

        Ok(())
    }

    pub(crate) async fn store_unavailable_guilds(
        &self,
        pipe: &mut Pipe<'_, C>,
        unavailable_guilds: &[UnavailableGuild],
    ) -> CacheResult<()> {
        let guild_ids: Vec<_> = unavailable_guilds
            .iter()
            .map(|guild| guild.id.get())
            .collect();

        self.delete_guilds(pipe, &guild_ids).await?;

        let key = RedisKey::UnavailableGuilds;
        pipe.sadd(key, guild_ids.as_slice()).ignore();

        Ok(())
    }
}