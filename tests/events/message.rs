use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    time::Duration,
};

use futures_util::TryStreamExt;
use redlight::{
    config::{CacheConfig, Cacheable, ICachedMessage, Ignore, ReactionEvent},
    error::BoxedError,
    rkyv_util::util::{BitflagsRkyv, RkyvAsU8},
    CacheError, CachedArchive, RedisCache,
};
use rkyv::{ser::serializers::BufferSerializer, with::Map, AlignedBytes, Archive, Serialize};
use serial_test::serial;
use twilight_model::{
    channel::{
        message::{
            sticker::{MessageSticker, StickerFormatType},
            Mention, MessageActivity, MessageActivityType, MessageFlags, MessageType, Reaction,
            ReactionType, RoleSubscriptionData,
        },
        ChannelMention, ChannelType, Message,
    },
    gateway::{
        event::Event,
        payload::incoming::{MessageCreate, MessageUpdate},
    },
    id::Id,
    user::UserFlags,
    util::Timestamp,
};

use crate::pool;

use super::{member::partial_member, user::user};

#[tokio::test]
#[serial]
async fn test_message() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        #[cfg(feature = "metrics")]
        const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(60);

        type Channel<'a> = Ignore;
        type CurrentUser<'a> = Ignore;
        type Emoji<'a> = Ignore;
        type Guild<'a> = Ignore;
        type Integration<'a> = Ignore;
        type Member<'a> = Ignore;
        type Message<'a> = CachedMessage;
        type Presence<'a> = Ignore;
        type Role<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedMessage {
        #[with(Map<BitflagsRkyv>)]
        flags: Option<MessageFlags>,
        #[with(RkyvAsU8)]
        kind: MessageType,
        timestamp: i64,
    }

    impl<'a> ICachedMessage<'a> for CachedMessage {
        fn from_message(message: &'a Message) -> Self {
            Self {
                flags: message.flags,
                kind: message.kind,
                timestamp: message.timestamp.as_micros(),
            }
        }

        fn on_message_update(
        ) -> Option<fn(&mut CachedArchive<Self>, &MessageUpdate) -> Result<(), BoxedError>>
        {
            Some(|archived, update| {
                archived.update_archive(|mut pinned| {
                    if let Some(kind) = update.kind {
                        pinned.kind = u8::from(kind);

                        // the `.into()` is necessary in case the `archive_le` or `archive_be`
                        // features are enabled in rkyv
                        #[allow(clippy::useless_conversion)]
                        if let Some(timestamp) = update.timestamp {
                            pinned.timestamp = timestamp.as_micros().into();
                        }
                    }
                });

                Ok(())
            })
        }

        fn on_reaction_event(
        ) -> Option<fn(&mut CachedArchive<Self>, ReactionEvent<'_>) -> Result<(), BoxedError>>
        {
            None
        }
    }

    impl Cacheable for CachedMessage {
        type Serializer = BufferSerializer<AlignedBytes<32>>;

        fn expire() -> Option<Duration> {
            None
        }
    }

    impl PartialEq<Message> for ArchivedCachedMessage {
        fn eq(&self, other: &Message) -> bool {
            self.flags == other.flags.map(|flags| flags.bits()) && self.kind == u8::from(other.kind)
        }
    }

    impl Debug for ArchivedCachedMessage {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            f.debug_struct("ArchivedCachedMessage")
                .field("flags", &self.flags)
                .field("kind", &self.kind)
                .finish()
        }
    }

    let cache = RedisCache::<Config>::new_with_pool(pool()).await?;

    let mut expected = message();
    expected.timestamp = Timestamp::from_micros(1_234_456_780).unwrap();

    let message_create = Event::MessageCreate(Box::new(MessageCreate(expected.clone())));
    cache.update(&message_create).await?;

    let message = cache.message(expected.id).await?.expect("missing message");

    assert_eq!(message.deref(), &expected);

    let update = message_update();
    expected.kind = update.kind.unwrap();

    let message_update = Event::MessageUpdate(Box::new(update));
    cache.update(&message_update).await?;

    let message = cache.message(expected.id).await?.expect("missing message");

    assert_eq!(message.deref(), &expected);

    // more recent message
    expected.id = Id::new(expected.id.get() + 1);
    expected.timestamp = Timestamp::from_secs(123_456_789).unwrap();
    expected.flags = Some(MessageFlags::empty());

    let message_create = Event::MessageCreate(Box::new(MessageCreate(expected.clone())));
    cache.update(&message_create).await?;

    // older message
    expected.id = Id::new(expected.id.get() + 1);
    expected.timestamp = Timestamp::from_secs(12_345_678_901).unwrap();
    expected.flags = Some(MessageFlags::all());

    let message_create = Event::MessageCreate(Box::new(MessageCreate(expected.clone())));
    cache.update(&message_create).await?;

    let messages: Vec<_> = cache
        .iter()
        .channel_messages(expected.channel_id)
        .await?
        .try_collect()
        .await?;

    assert_eq!(messages.len(), 3);

    let is_sorted = messages.windows(2).all(|window| {
        let [a, b] = window else { unreachable!() };

        a.flags != b.flags && a.timestamp > b.timestamp
    });

    assert!(is_sorted);

    Ok(())
}

pub fn message() -> Message {
    Message {
        activity: Some(MessageActivity {
            kind: MessageActivityType::Join,
            party_id: None,
        }),
        application: None,
        application_id: None,
        attachments: Vec::new(),
        author: user(),
        channel_id: Id::new(222),
        components: Vec::new(),
        content: "message content".to_owned(),
        edited_timestamp: None,
        embeds: Vec::new(),
        flags: Some(MessageFlags::URGENT),
        guild_id: Some(Id::new(111)),
        id: Id::new(909),
        interaction: None,
        kind: MessageType::Regular,
        member: Some(partial_member()),
        mention_channels: vec![ChannelMention {
            guild_id: Id::new(667),
            id: Id::new(668),
            kind: ChannelType::GuildText,
            name: "channel name".to_owned(),
        }],
        mention_everyone: false,
        mention_roles: vec![Id::new(456), Id::new(567)],
        mentions: vec![Mention {
            avatar: None,
            bot: false,
            discriminator: 1234,
            id: partial_member().user.unwrap().id,
            member: Some(partial_member()),
            name: "mention name".to_owned(),
            public_flags: UserFlags::ACTIVE_DEVELOPER,
        }],
        pinned: false,
        reactions: vec![Reaction {
            count: 1,
            emoji: ReactionType::Unicode {
                name: "🍕".to_owned(),
            },
            me: false,
        }],
        reference: None,
        referenced_message: None,
        role_subscription_data: Some(RoleSubscriptionData {
            is_renewal: true,
            role_subscription_listing_id: Id::new(100),
            tier_name: "tier name".to_owned(),
            total_months_subscribed: 13,
        }),
        sticker_items: vec![MessageSticker {
            format_type: StickerFormatType::Apng,
            id: Id::new(78),
            name: "sticker name".to_owned(),
        }],
        timestamp: Timestamp::parse("2021-01-01T01:01:01+00:00").unwrap(),
        thread: None,
        tts: false,
        webhook_id: None,
    }
}

pub fn message_update() -> MessageUpdate {
    let msg = message();

    MessageUpdate {
        attachments: None,
        author: None,
        channel_id: msg.channel_id,
        content: None,
        edited_timestamp: None,
        embeds: None,
        guild_id: None,
        id: msg.id,
        kind: Some(MessageType::Call),
        mention_everyone: None,
        mention_roles: None,
        mentions: None,
        pinned: None,
        timestamp: None,
        tts: None,
    }
}
