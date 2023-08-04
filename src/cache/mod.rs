mod delete;
mod get;
mod pipe;
mod store;

use std::marker::PhantomData;

use twilight_model::gateway::event::Event;

use crate::{
    cache::pipe::Pipe,
    config::CacheConfig,
    iter::RedisCacheIter,
    redis::{Connection, Pool},
    CacheError, CacheResult,
};

/// Redis-based cache for data of twilight's gateway [`Event`]s.
pub struct RedisCache<C> {
    pool: Pool,
    config: PhantomData<C>,
}

impl<C> RedisCache<C> {
    #[cfg(feature = "bb8")]
    pub async fn new(url: &str) -> CacheResult<Self> {
        use bb8_redis::RedisConnectionManager;

        let manager = RedisConnectionManager::new(url).map_err(CacheError::CreatePool)?;

        let pool = Pool::builder()
            .build(manager)
            .await
            .map_err(CacheError::CreatePool)?;

        Ok(Self::with_pool(pool))
    }

    #[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
    pub async fn new(url: &str) -> CacheResult<Self> {
        use deadpool_redis::{Config, Runtime};

        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        Ok(Self::with_pool(pool))
    }

    #[cfg(any(feature = "bb8", feature = "deadpool"))]
    pub fn with_pool(pool: Pool) -> Self {
        Self {
            pool,
            config: PhantomData,
        }
    }

    pub fn iter(&self) -> RedisCacheIter<'_, C> {
        RedisCacheIter::new(self)
    }

    pub(crate) async fn connection(&self) -> CacheResult<Connection<'_>> {
        Connection::get(&self.pool)
            .await
            .map_err(CacheError::GetConnection)
    }
}

impl<C: CacheConfig> RedisCache<C> {
    pub async fn update(&self, event: &Event) -> CacheResult<()> {
        let start = std::time::Instant::now();

        let mut pipe = Pipe::new(self);

        match event {
            Event::AutoModerationActionExecution(_) => {}
            Event::AutoModerationRuleCreate(_) => {}
            Event::AutoModerationRuleDelete(_) => {}
            Event::AutoModerationRuleUpdate(_) => {}
            Event::BanAdd(event) => self.store_user(&mut pipe, &event.user)?,
            Event::BanRemove(event) => self.store_user(&mut pipe, &event.user)?,
            Event::ChannelCreate(event) => self.store_channel(&mut pipe, event)?,
            Event::ChannelDelete(event) => self.delete_channel(&mut pipe, event.guild_id, event.id),
            Event::ChannelPinsUpdate(event) => {
                self.store_channel_pins_update(&mut pipe, event).await?
            }
            Event::ChannelUpdate(event) => self.store_channel(&mut pipe, event)?,
            Event::CommandPermissionsUpdate(_) => {}
            Event::GatewayClose(_) => {}
            Event::GatewayHeartbeat(_) => {}
            Event::GatewayHeartbeatAck => {}
            Event::GatewayHello(_) => {}
            Event::GatewayInvalidateSession(_) => {}
            Event::GatewayReconnect => {}
            Event::GiftCodeUpdate => {}
            Event::GuildAuditLogEntryCreate(_) => {}
            Event::GuildCreate(event) => self.store_guild(&mut pipe, event)?,
            Event::GuildDelete(event) => {
                if event.unavailable {
                    self.store_unavailable_guild(&mut pipe, event.id).await?
                } else {
                    self.delete_guild(&mut pipe, event.id).await?
                }
            }
            Event::GuildEmojisUpdate(event) => {
                self.store_emojis(&mut pipe, event.guild_id, &event.emojis)?
            }
            Event::GuildIntegrationsUpdate(_) => {}
            Event::GuildScheduledEventCreate(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(&mut pipe, user)?;
                }
            }
            Event::GuildScheduledEventDelete(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(&mut pipe, user)?;
                }
            }
            Event::GuildScheduledEventUpdate(event) => {
                if let Some(ref user) = event.creator {
                    self.store_user(&mut pipe, user)?;
                }
            }
            Event::GuildScheduledEventUserAdd(_) => {}
            Event::GuildScheduledEventUserRemove(_) => {}
            Event::GuildStickersUpdate(event) => {
                self.store_stickers(&mut pipe, event.guild_id, &event.stickers)?
            }
            Event::GuildUpdate(event) => self.store_guild_update(&mut pipe, event).await?,
            Event::IntegrationCreate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(&mut pipe, guild_id, event)?;
                }
            }
            Event::IntegrationDelete(event) => {
                self.delete_integration(&mut pipe, event.guild_id, event.id)
            }
            Event::IntegrationUpdate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(&mut pipe, guild_id, event)?;
                }
            }
            Event::InteractionCreate(event) => self.store_interaction(&mut pipe, event).await?,
            Event::InviteCreate(event) => {
                if let Some(ref user) = event.inviter {
                    self.store_user(&mut pipe, user)?;
                }

                if let Some(ref user) = event.target_user {
                    self.store_partial_user(&mut pipe, user).await?;
                }
            }
            Event::InviteDelete(_) => {}
            Event::MemberAdd(event) => {
                self.store_member(&mut pipe, event.guild_id, &event.member)?
            }
            Event::MemberRemove(event) => {
                self.delete_member(&mut pipe, event.guild_id, event.user.id)
                    .await?
            }
            Event::MemberUpdate(event) => self.store_member_update(&mut pipe, event).await?,
            Event::MemberChunk(event) => {
                self.store_members(&mut pipe, event.guild_id, &event.members)?;
                self.store_presences(&mut pipe, event.guild_id, &event.presences)?;
            }
            Event::MessageCreate(event) => self.store_message(&mut pipe, event).await?,
            Event::MessageDelete(event) => self.delete_message(&mut pipe, event.id),
            Event::MessageDeleteBulk(event) => self.delete_messages(&mut pipe, &event.ids),
            Event::MessageUpdate(event) => self.store_message_update(&mut pipe, event).await?,
            Event::PresenceUpdate(event) => self.store_presence(&mut pipe, event)?,
            Event::PresencesReplace => {}
            Event::ReactionAdd(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member)?;
                }
            }
            Event::ReactionRemove(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member)?;
                }
            }
            Event::ReactionRemoveAll(_) => {}
            Event::ReactionRemoveEmoji(_) => {}
            Event::Ready(event) => {
                self.store_unavailable_guilds(&mut pipe, &event.guilds)
                    .await?;
                self.store_current_user(&mut pipe, &event.user)?;
            }
            Event::Resumed => {}
            Event::RoleCreate(event) => self.store_role(&mut pipe, event.guild_id, &event.role)?,
            Event::RoleDelete(event) => self.delete_role(&mut pipe, event.guild_id, event.role_id),
            Event::RoleUpdate(event) => self.store_role(&mut pipe, event.guild_id, &event.role)?,
            Event::StageInstanceCreate(event) => self.store_stage_instance(&mut pipe, event)?,
            Event::StageInstanceDelete(event) => {
                self.delete_stage_instance(&mut pipe, event.guild_id, event.id)
            }
            Event::StageInstanceUpdate(event) => self.store_stage_instance(&mut pipe, event)?,
            Event::ThreadCreate(event) => self.store_channel(&mut pipe, event)?,
            Event::ThreadDelete(event) => {
                self.delete_channel(&mut pipe, Some(event.guild_id), event.id)
            }
            Event::ThreadListSync(event) => {
                self.store_channels(&mut pipe, event.guild_id, &event.threads)?
            }
            Event::ThreadMemberUpdate(event) => {
                if let Some(ref presence) = event.presence {
                    self.store_presence(&mut pipe, presence)?;

                    if let Some(ref member) = event.member {
                        self.store_member(&mut pipe, presence.guild_id, member)?;
                    }
                }
            }
            Event::ThreadMembersUpdate(_) => {}
            Event::ThreadUpdate(event) => self.store_channel(&mut pipe, event)?,
            Event::TypingStart(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member)?;
                }
            }
            Event::UnavailableGuild(event) => {
                self.store_unavailable_guild(&mut pipe, event.id).await?
            }
            Event::UserUpdate(event) => self.store_current_user(&mut pipe, event)?,
            Event::VoiceServerUpdate(_) => {}
            Event::VoiceStateUpdate(event) => {
                if let Some(channel_id) = event.channel_id {
                    self.store_voice_state(&mut pipe, channel_id, event)?;
                } else if let Some(guild_id) = event.guild_id {
                    self.delete_voice_state(&mut pipe, guild_id, event.user_id);
                }
            }
            Event::WebhooksUpdate(_) => {}
        };

        if !pipe.is_empty() {
            pipe.query::<()>().await?;
        }

        let elapsed = start.elapsed();
        println!("{:?}: {elapsed:.2?}", event.kind());

        Ok(())
    }
}
