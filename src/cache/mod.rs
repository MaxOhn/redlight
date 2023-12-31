mod cold_resume;
mod expire;
mod get;
mod impls;
mod meta;
mod pipe;

use std::marker::PhantomData;

use tracing::instrument;
use twilight_model::gateway::event::Event;

use crate::{
    cache::pipe::Pipe,
    config::{CacheConfig, ReactionEvent},
    iter::RedisCacheIter,
    redis::{Connection, Pool},
    stats::RedisCacheStats,
    CacheError, CacheResult,
};

/// Redis-based cache for data of twilight's gateway [`Event`]s.
pub struct RedisCache<C> {
    pool: Pool,
    config: PhantomData<C>,
}

impl<C> RedisCache<C> {
    pub(crate) async fn connection(&self) -> CacheResult<Connection<'_>> {
        Connection::get(&self.pool)
            .await
            .map_err(CacheError::GetConnection)
    }

    /// Create a [`RedisCacheIter`] instance to iterate over various cached collections.
    #[allow(clippy::iter_not_returning_iterator)]
    pub const fn iter(&self) -> RedisCacheIter<'_, C> {
        RedisCacheIter::new(self)
    }

    /// Create a [`RedisCacheStats`] instance to inspect sizes of cached collections.
    pub const fn stats(&self) -> RedisCacheStats<'_, C> {
        RedisCacheStats::new(self)
    }
}

impl<C: CacheConfig> RedisCache<C> {
    #[cfg(feature = "bb8")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "bb8", feature = "deadpool"))))]
    /// Create a new [`RedisCache`].
    ///
    /// The cache will connect to a new default bb8 connection pool through the given url.
    pub async fn new(url: &str) -> CacheResult<Self> {
        use bb8_redis::RedisConnectionManager;

        let manager = RedisConnectionManager::new(url).map_err(CacheError::CreatePool)?;

        let pool = Pool::builder()
            .build(manager)
            .await
            .map_err(CacheError::CreatePool)?;

        Self::new_with_pool(pool).await
    }

    #[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "bb8", feature = "deadpool"))))]
    /// Create a new [`RedisCache`].
    ///
    /// The cache will connect to a new default deadpool connection pool through the given url.
    pub async fn new(url: &str) -> CacheResult<Self> {
        use deadpool_redis::{Config, Runtime};

        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        Self::with_pool(pool).await
    }

    #[cfg(any(feature = "bb8", feature = "deadpool"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "bb8", feature = "deadpool"))))]
    /// Create a new [`RedisCache`] by using the given connection pool.
    ///
    /// This provides a way to customize the pool configuration manually.
    pub async fn new_with_pool(pool: Pool) -> CacheResult<Self> {
        Self::handle_expire(&pool).await?;

        #[cfg(feature = "metrics")]
        Self::init_metrics(&pool);

        Ok(Self {
            pool,
            config: PhantomData,
        })
    }

    #[cfg(any(feature = "bb8", feature = "deadpool"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "bb8", feature = "deadpool"))))]
    /// Get a reference to the underlying redis connection pool.
    pub const fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Update the cache with an [`Event`] from the gateway.
    #[instrument(skip_all, fields(event = ?event.kind()))]
    pub async fn update(&self, event: &Event) -> CacheResult<()> {
        let mut pipe = Pipe::new(self);

        #[allow(clippy::match_same_arms)]
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
                self.store_channel_pins_update(&mut pipe, event).await?;
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
                    self.store_unavailable_guild(&mut pipe, event.id).await?;
                } else {
                    self.delete_guild(&mut pipe, event.id).await?;
                }
            }
            Event::GuildEmojisUpdate(event) => {
                self.store_emojis(&mut pipe, event.guild_id, &event.emojis)?;
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
                self.store_stickers(&mut pipe, event.guild_id, &event.stickers)?;
            }
            Event::GuildUpdate(event) => self.store_guild_update(&mut pipe, event).await?,
            Event::IntegrationCreate(event) => {
                if let Some(guild_id) = event.guild_id {
                    self.store_integration(&mut pipe, guild_id, event)?;
                }
            }
            Event::IntegrationDelete(event) => {
                self.delete_integration(&mut pipe, event.guild_id, event.id);
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
                self.store_member(&mut pipe, event.guild_id, &event.member)?;
            }
            Event::MemberRemove(event) => {
                self.delete_member(&mut pipe, event.guild_id, event.user.id)
                    .await?;
            }
            Event::MemberUpdate(event) => self.store_member_update(&mut pipe, event).await?,
            Event::MemberChunk(event) => {
                self.store_members(&mut pipe, event.guild_id, &event.members)?;
                self.store_presences(&mut pipe, event.guild_id, &event.presences)?;
            }
            Event::MessageCreate(event) => self.store_message(&mut pipe, event).await?,
            Event::MessageDelete(event) => {
                self.delete_message(&mut pipe, event.id, event.channel_id);
            }
            Event::MessageDeleteBulk(event) => {
                self.delete_messages(&mut pipe, &event.ids, event.channel_id);
            }
            Event::MessageUpdate(event) => self.store_message_update(&mut pipe, event).await?,
            Event::PresenceUpdate(event) => self.store_presence(&mut pipe, event)?,
            Event::PresencesReplace => {}
            Event::ReactionAdd(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member)?;
                }

                self.handle_reaction(&mut pipe, ReactionEvent::Add(event))
                    .await?;
            }
            Event::ReactionRemove(event) => {
                if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
                    self.store_member(&mut pipe, guild_id, member)?;
                }

                self.handle_reaction(&mut pipe, ReactionEvent::Remove(event))
                    .await?;
            }
            Event::ReactionRemoveAll(event) => {
                self.handle_reaction(&mut pipe, ReactionEvent::RemoveAll(event))
                    .await?;
            }
            Event::ReactionRemoveEmoji(event) => {
                self.handle_reaction(&mut pipe, ReactionEvent::RemoveEmoji(event))
                    .await?;
            }
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
                self.delete_stage_instance(&mut pipe, event.guild_id, event.id);
            }
            Event::StageInstanceUpdate(event) => self.store_stage_instance(&mut pipe, event)?,
            Event::ThreadCreate(event) => self.store_channel(&mut pipe, event)?,
            Event::ThreadDelete(event) => {
                self.delete_channel(&mut pipe, Some(event.guild_id), event.id);
            }
            Event::ThreadListSync(event) => {
                self.store_channels(&mut pipe, event.guild_id, &event.threads)?;
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
                self.store_unavailable_guild(&mut pipe, event.id).await?;
            }
            Event::UserUpdate(event) => self.store_current_user(&mut pipe, event)?,
            Event::VoiceServerUpdate(_) => {}
            Event::VoiceStateUpdate(event) => {
                if let Some(guild_id) = event.guild_id {
                    if let Some(channel_id) = event.channel_id {
                        self.store_voice_state(&mut pipe, channel_id, guild_id, event)?;
                    } else {
                        self.delete_voice_state(&mut pipe, guild_id, event.user_id);
                    }
                }
            }
            Event::WebhooksUpdate(_) => {}
        };

        if !pipe.is_empty() {
            pipe.query::<()>().await?;
        }

        Ok(())
    }

    #[cfg(feature = "metrics")]
    fn init_metrics(pool: &Pool) {
        use crate::config::Cacheable;

        let wants_any = C::Channel::WANTED
            || C::Emoji::WANTED
            || C::Guild::WANTED
            || C::Message::WANTED
            || C::Role::WANTED
            || C::StageInstance::WANTED
            || C::Sticker::WANTED
            || C::User::WANTED;

        if !wants_any {
            return;
        }

        tokio::spawn(metrics_loop::<C>(pool.clone()));
    }
}

#[cfg(feature = "metrics")]
async fn metrics_loop<C: CacheConfig>(pool: Pool) {
    use metrics::{describe_gauge, gauge};
    use tracing::{error, trace};

    use crate::{config::Cacheable, key::RedisKey, redis::Pipeline};

    const CHANNEL_COUNT: &str = "channel_count";
    const EMOJI_COUNT: &str = "emoji_count";
    const GUILD_COUNT: &str = "guild_count";
    const MESSAGE_COUNT: &str = "message_count";
    const ROLE_COUNT: &str = "role_count";
    const STAGE_INSTANCE_COUNT: &str = "stage_instance_count";
    const STICKER_COUNT: &str = "sticker_count";
    const UNAVAILABLE_GUILD_COUNT: &str = "unavailable_guild_count";
    const USER_COUNT: &str = "user_count";

    describe_gauge!(CHANNEL_COUNT, "Amount of cached channels");
    describe_gauge!(EMOJI_COUNT, "Amount of cached emojis");
    describe_gauge!(GUILD_COUNT, "Amount of cached guilds");
    describe_gauge!(MESSAGE_COUNT, "Amount of cached messages");
    describe_gauge!(ROLE_COUNT, "Amount of cached roles");
    describe_gauge!(STAGE_INSTANCE_COUNT, "Amount of cached stage instances");
    describe_gauge!(STICKER_COUNT, "Amount of cached stickers");
    describe_gauge!(UNAVAILABLE_GUILD_COUNT, "Amount of unavailable guilds");
    describe_gauge!(USER_COUNT, "Amount of cached users");

    let duration = C::METRICS_INTERVAL_DURATION;
    let mut pipe = Pipeline::new();
    let mut interval = tokio::time::interval(duration);

    trace!(interval = ?duration, "Running metrics loop");

    interval.tick().await;

    loop {
        interval.tick().await;

        if C::Channel::WANTED {
            pipe.scard(RedisKey::Channels);
        }

        if C::Emoji::WANTED {
            pipe.scard(RedisKey::Emojis);
        }

        if C::Guild::WANTED {
            pipe.scard(RedisKey::Guilds);
            pipe.scard(RedisKey::UnavailableGuilds);
        }

        if C::Message::WANTED {
            pipe.scard(RedisKey::Messages);
        }

        if C::Role::WANTED {
            pipe.scard(RedisKey::Roles);
        }

        if C::StageInstance::WANTED {
            pipe.scard(RedisKey::StageInstances);
        }

        if C::Sticker::WANTED {
            pipe.scard(RedisKey::Stickers);
        }

        if C::User::WANTED {
            pipe.scard(RedisKey::Users);
        }

        let mut conn = match Connection::get(&pool).await {
            Ok(conn) => conn,
            Err(err) => {
                error!(%err, "Failed to acquire connection for metrics");

                continue;
            }
        };

        let mut scards = match pipe.query_async::<_, Vec<usize>>(&mut conn).await {
            Ok(scards) => scards.into_iter(),
            Err(err) => {
                error!(%err, "Failed to request metric values from redis");

                continue;
            }
        };

        pipe.clear();

        #[allow(clippy::cast_precision_loss)]
        let mut next_scard = || scards.next().unwrap_or(0) as f64;

        if C::Channel::WANTED {
            gauge!(CHANNEL_COUNT, next_scard());
        }

        if C::Emoji::WANTED {
            gauge!(EMOJI_COUNT, next_scard());
        }

        if C::Guild::WANTED {
            gauge!(GUILD_COUNT, next_scard());
            gauge!(UNAVAILABLE_GUILD_COUNT, next_scard());
        }

        if C::Message::WANTED {
            gauge!(MESSAGE_COUNT, next_scard());
        }

        if C::Role::WANTED {
            gauge!(ROLE_COUNT, next_scard());
        }

        if C::StageInstance::WANTED {
            gauge!(STAGE_INSTANCE_COUNT, next_scard());
        }

        if C::Sticker::WANTED {
            gauge!(STICKER_COUNT, next_scard());
        }

        if C::User::WANTED {
            gauge!(USER_COUNT, next_scard());
        }
    }
}
