use std::{
    error::Error as StdError,
    fmt::{Debug, Formatter, Result as FmtResult},
};

use rkyv::{ser::Serializer, Archived, Serialize};
use tracing::{instrument, trace};
use twilight_model::id::Id;

use crate::{
    config::checked::CheckedArchive,
    error::ExpireError,
    key::RedisKey,
    redis::{DedicatedConnection, Pipeline},
    ser::CacheSerializer,
};

use super::{
    impls::{
        channel::ChannelMetaKey, emoji::EmojiMetaKey, guild::GuildMetaKey,
        integration::IntegrationMetaKey, member::MemberMetaKey, message::MessageMetaKey,
        presence::PresenceMetaKey, role::RoleMetaKey, stage_instance::StageInstanceMetaKey,
        sticker::StickerMetaKey, user::UserMetaKey, voice_state::VoiceStateMetaKey,
    },
    pipe::Pipe,
};

pub(crate) enum MetaKey {
    Channel(ChannelMetaKey),
    Emoji(EmojiMetaKey),
    Guild(GuildMetaKey),
    Integration(IntegrationMetaKey),
    Member(MemberMetaKey),
    Message(MessageMetaKey),
    Presence(PresenceMetaKey),
    Role(RoleMetaKey),
    StageInstance(StageInstanceMetaKey),
    Sticker(StickerMetaKey),
    User(UserMetaKey),
    VoiceState(VoiceStateMetaKey),
}

impl MetaKey {
    pub(crate) fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        match split.next() {
            Some(RedisKey::CHANNEL_PREFIX) => IMetaKey::parse(split).map(Self::Channel),
            Some(RedisKey::EMOJI_PREFIX) => IMetaKey::parse(split).map(Self::Emoji),
            Some(RedisKey::GUILD_PREFIX) => IMetaKey::parse(split).map(Self::Guild),
            Some(RedisKey::INTEGRATION_PREFIX) => IMetaKey::parse(split).map(Self::Integration),
            Some(RedisKey::MEMBER_PREFIX) => IMetaKey::parse(split).map(Self::Member),
            Some(RedisKey::MESSAGE_PREFIX) => IMetaKey::parse(split).map(Self::Message),
            Some(RedisKey::PRESENCE_PREFIX) => IMetaKey::parse(split).map(Self::Presence),
            Some(RedisKey::ROLE_PREFIX) => IMetaKey::parse(split).map(Self::Role),
            Some(RedisKey::STAGE_INSTANCE_PREFIX) => {
                IMetaKey::parse(split).map(Self::StageInstance)
            }
            Some(RedisKey::STICKER_PREFIX) => IMetaKey::parse(split).map(Self::Sticker),
            Some(RedisKey::USER_PREFIX) => IMetaKey::parse(split).map(Self::User),
            Some(RedisKey::VOICE_STATE_PREFIX) => IMetaKey::parse(split).map(Self::VoiceState),
            Some(_) | None => None,
        }
    }

    #[instrument(level = "trace", skip(conn, pipe))]
    pub(crate) async fn handle_expire(
        self,
        conn: &mut DedicatedConnection,
        pipe: &mut Pipeline,
    ) -> Result<(), ExpireError> {
        match self {
            MetaKey::Channel(meta) => {
                let key = meta.redis_key();

                let Some(bytes) = Self::fetch_bytes(conn, pipe, key).await? else {
                    return Ok(());
                };

                let archived = <ChannelMetaKey as HasArchived>::Meta::as_archive(&bytes)?;
                meta.handle_archived(pipe, archived);
                meta.handle_expire(pipe);
            }
            MetaKey::Emoji(meta) => {
                let key = meta.redis_key();

                let Some(bytes) = Self::fetch_bytes(conn, pipe, key).await? else {
                    return Ok(());
                };

                let archived = <EmojiMetaKey as HasArchived>::Meta::as_archive(&bytes)?;
                meta.handle_archived(pipe, archived);
                meta.handle_expire(pipe);
            }
            MetaKey::Guild(meta) => {
                meta.handle_expire(pipe);
                meta.async_handle_expire(pipe, conn).await?;
            }
            MetaKey::Integration(meta) => meta.handle_expire(pipe),
            MetaKey::Member(meta) => {
                meta.handle_expire(pipe);
                meta.async_handle_expire(pipe, conn).await?;
            }
            MetaKey::Message(meta) => {
                let key = meta.redis_key();

                let Some(bytes) = Self::fetch_bytes(conn, pipe, key).await? else {
                    return Ok(());
                };

                let archived = <MessageMetaKey as HasArchived>::Meta::as_archive(&bytes)?;
                meta.handle_archived(pipe, archived);
                meta.handle_expire(pipe);
            }
            MetaKey::Presence(meta) => meta.handle_expire(pipe),
            MetaKey::Role(meta) => {
                let key = meta.redis_key();

                let Some(bytes) = Self::fetch_bytes(conn, pipe, key).await? else {
                    return Ok(());
                };

                let archived = <RoleMetaKey as HasArchived>::Meta::as_archive(&bytes)?;
                meta.handle_archived(pipe, archived);
                meta.handle_expire(pipe);
            }
            MetaKey::StageInstance(meta) => {
                let key = meta.redis_key();

                let Some(bytes) = Self::fetch_bytes(conn, pipe, key).await? else {
                    return Ok(());
                };

                let archived = <StageInstanceMetaKey as HasArchived>::Meta::as_archive(&bytes)?;
                meta.handle_archived(pipe, archived);
                meta.handle_expire(pipe);
            }
            MetaKey::Sticker(meta) => {
                let key = meta.redis_key();

                let Some(bytes) = Self::fetch_bytes(conn, pipe, key).await? else {
                    return Ok(());
                };

                let archived = <StickerMetaKey as HasArchived>::Meta::as_archive(&bytes)?;
                meta.handle_archived(pipe, archived);
                meta.handle_expire(pipe);
            }
            MetaKey::User(meta) => meta.handle_expire(pipe),
            MetaKey::VoiceState(meta) => meta.handle_expire(pipe),
        }

        trace!(piped = pipe.cmd_iter().count());

        Ok(())
    }

    async fn fetch_bytes(
        conn: &mut DedicatedConnection,
        pipe: &mut Pipeline,
        key: RedisKey,
    ) -> Result<Option<Vec<u8>>, ExpireError> {
        debug_assert_eq!(pipe.cmd_iter().count(), 0);

        let res = pipe
            .get_del(key)
            .query_async::<_, Option<Vec<u8>>>(conn)
            .await
            .map(|opt| opt.filter(|bytes| !bytes.is_empty()))
            .map_err(ExpireError::GetMeta);

        pipe.clear();

        res
    }
}

impl Debug for MetaKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Channel(meta) => Debug::fmt(meta, f),
            Self::Emoji(meta) => Debug::fmt(meta, f),
            Self::Guild(meta) => Debug::fmt(meta, f),
            Self::Integration(meta) => Debug::fmt(meta, f),
            Self::Member(meta) => Debug::fmt(meta, f),
            Self::Message(meta) => Debug::fmt(meta, f),
            Self::Presence(meta) => Debug::fmt(meta, f),
            Self::Role(meta) => Debug::fmt(meta, f),
            Self::StageInstance(meta) => Debug::fmt(meta, f),
            Self::Sticker(meta) => Debug::fmt(meta, f),
            Self::User(meta) => Debug::fmt(meta, f),
            Self::VoiceState(meta) => Debug::fmt(meta, f),
        }
    }
}

/// All the data given by a [`RedisKey`] alone.
///
/// Created from an expire payload. If additional data is required to perform
/// the expire cleanup, implement [`HasArchived`].
pub(crate) trait IMetaKey: Sized {
    /// Parse from an expire payload.
    fn parse<'a>(split: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self>;

    /// What to do after the payload has been parsed.
    fn handle_expire(&self, pipe: &mut Pipeline);
}

/// Specifies that a [`IMetaKey`] has additional archived data.
pub(crate) trait HasArchived: Sized {
    type Meta: IMeta<Self>;

    /// The [`RedisKey`] to gather the additional data.
    fn redis_key(&self) -> RedisKey;

    /// What to do after the additional data has been retrieved.
    fn handle_archived(&self, pipe: &mut Pipeline, archived: &Archived<Self::Meta>);
}

/// Additional data for a [`IMetaKey`] that gets archived in the cache.
pub(crate) trait IMeta<Key: HasArchived>: CheckedArchive + Sized
where
    Self: Serialize<Self::Serializer>,
{
    /// The serializer to serialize this data.
    type Serializer: CacheSerializer;

    /// Serialize and store this data in the cache.
    fn store<C>(&self, pipe: &mut Pipe<'_, C>, key: Key) -> Result<(), Box<dyn StdError>> {
        let mut serializer = Self::Serializer::default();
        serializer.serialize_value(self)?;
        let bytes = serializer.finish();
        let key = key.redis_key();
        pipe.set(key, bytes.as_ref(), None);

        Ok(())
    }

    /// Interprete the given bytes as an archived type.
    fn as_archive(bytes: &[u8]) -> Result<&Archived<Self>, ExpireError> {
        #[cfg(feature = "validation")]
        {
            rkyv::check_archived_root::<Self>(bytes)
                .map_err(|e| ExpireError::Validation(Box::new(e)))
        }

        #[cfg(not(feature = "validation"))]
        unsafe {
            Ok(rkyv::archived_root::<Self>(bytes))
        }
    }
}

/// Parse a slice into an [`Id<T>`].
pub(super) fn atoi<T>(bytes: &[u8]) -> Option<Id<T>> {
    bytes
        .iter()
        .try_fold(0_u64, |n, byte| {
            if !byte.is_ascii_digit() {
                return None;
            }

            n.checked_mul(10)?.checked_add((*byte & 0xF) as u64)
        })
        .and_then(Id::new_checked)
}
