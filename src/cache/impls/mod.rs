pub(super) mod channel;
pub(super) mod current_user;
pub(super) mod emoji;
pub(super) mod guild;
pub(super) mod integration;
pub(super) mod member;
pub(super) mod message;
pub(super) mod presence;
pub(super) mod role;
pub(super) mod scheduled_event;
pub(super) mod stage_instance;
pub(super) mod sticker;
pub(super) mod user;
pub(super) mod voice_state;

use tracing::instrument;
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    guild::UnavailableGuild,
    id::{marker::GuildMarker, Id},
};

use super::pipe::Pipe;
use crate::{
    config::{CacheConfig, Cacheable},
    key::RedisKey,
    CacheResult, RedisCache,
};

impl<C: CacheConfig> RedisCache<C> {
    #[instrument(level = "trace", skip_all)]
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

    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn store_unavailable_guild(
        &self,
        pipe: &mut Pipe<'_, C>,
        guild_id: Id<GuildMarker>,
    ) -> CacheResult<()> {
        self.delete_guild(pipe, guild_id).await?;

        if C::Guild::WANTED {
            let key = RedisKey::UnavailableGuilds;
            pipe.sadd(key, guild_id.get());
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
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

        if C::Guild::WANTED {
            let key = RedisKey::UnavailableGuilds;
            pipe.sadd(key, guild_ids.as_slice());
        }

        Ok(())
    }
}
