use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
    time::Duration,
};

use redlight::{
    config::{CacheConfig, Cacheable, ICachedMessage, Ignore, ReactionEvent},
    error::CacheError,
    rkyv_util::{flags::BitflagsRkyv, util::RkyvAsU8},
    CachedArchive, RedisCache,
};
use rkyv::{
    rancor::Source, ser::writer::Buffer, util::Align, with::Map, Archive, Archived, Serialize,
};
use twilight_model::{
    channel::{
        message::{
            sticker::{MessageSticker, StickerFormatType},
            EmojiReactionType, Mention, MessageActivity, MessageActivityType, MessageFlags,
            MessageType, Reaction, ReactionCountDetails, RoleSubscriptionData,
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

use super::{member::partial_member, user::user};
use crate::pool;

#[tokio::test]
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
        type ScheduledEvent<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    struct CachedMessage {
        #[rkyv(with = Map<BitflagsRkyv>)]
        flags: Option<MessageFlags>,
        #[rkyv(with = RkyvAsU8)]
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

        fn on_message_update<E: Source>(
        ) -> Option<fn(&mut CachedArchive<Archived<Self>>, &MessageUpdate) -> Result<(), E>>
        {
            Some(|archived, update| {
                archived
                    .update_archive(|sealed| {
                        rkyv::munge::munge! {
                            let ArchivedCachedMessage { mut kind, mut timestamp, .. } = sealed
                        };

                        *kind = u8::from(update.kind);
                        *timestamp = update.timestamp.as_micros().into();
                    })
                    .map_err(Source::new)
            })
        }

        fn on_reaction_event<E: Source>(
        ) -> Option<fn(&mut CachedArchive<Archived<Self>>, ReactionEvent<'_>) -> Result<(), E>>
        {
            None
        }
    }

    impl Cacheable for CachedMessage {
        type Bytes = [u8; 32];

        fn expire() -> Option<Duration> {
            None
        }

        fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
            let mut bytes = Align([0_u8; 32]);
            rkyv::api::high::to_bytes_in(self, Buffer::from(&mut *bytes))?;

            Ok(bytes.0)
        }
    }

    impl PartialEq<Message> for ArchivedCachedMessage {
        fn eq(&self, other: &Message) -> bool {
            self.flags == other.flags && self.kind == u8::from(other.kind)
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
    expected.timestamp = Timestamp::from_micros(123_456_789).unwrap();

    let message_create = Event::MessageCreate(Box::new(MessageCreate(expected.clone())));
    cache.update(&message_create).await?;

    let message = cache.message(expected.id).await?.expect("missing message");

    assert_eq!(message.deref(), &expected);

    let update = message_update();
    expected.kind = update.kind;

    let message_update = Event::MessageUpdate(Box::new(update));
    cache.update(&message_update).await?;

    let message = cache.message(expected.id).await?.expect("missing message");

    assert_eq!(message.deref(), &expected);

    // more recent message
    expected.id = Id::new(expected.id.get() + 1);
    expected.timestamp = Timestamp::from_micros(1_234_567_899).unwrap();
    expected.flags = Some(MessageFlags::empty());

    let message_create = Event::MessageCreate(Box::new(MessageCreate(expected.clone())));
    cache.update(&message_create).await?;

    // older message
    expected.id = Id::new(expected.id.get() + 1);
    expected.timestamp = Timestamp::from_micros(1_234_567_890).unwrap();
    expected.flags = Some(MessageFlags::all());

    let message_create = Event::MessageCreate(Box::new(MessageCreate(expected.clone())));
    cache.update(&message_create).await?;

    let message_iter = cache.iter().channel_messages(expected.channel_id).await?;

    #[cfg(feature = "bytecheck")]
    let messages = message_iter.collect::<Result<Vec<_>, _>>()?;

    #[cfg(not(feature = "bytecheck"))]
    let messages = message_iter.collect::<Vec<_>>();

    assert_eq!(messages.len(), 3);

    let is_sorted = messages.windows(2).all(|window| {
        let [a, b] = window else { unreachable!() };

        a.flags != b.flags && a.timestamp > b.timestamp
    });

    assert!(is_sorted);

    Ok(())
}

pub fn message() -> Message {
    #[allow(deprecated)]
    Message {
        activity: Some(MessageActivity {
            kind: MessageActivityType::Join,
            party_id: None,
        }),
        application: None,
        application_id: None,
        attachments: Vec::new(),
        author: user(),
        call: None,
        channel_id: Id::new(222),
        components: Vec::new(),
        content: "message content".to_owned(),
        edited_timestamp: None,
        embeds: Vec::new(),
        flags: Some(MessageFlags::URGENT),
        guild_id: Some(Id::new(111)),
        id: Id::new(909),
        interaction: None,
        interaction_metadata: None,
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
        message_snapshots: Vec::new(),
        pinned: false,
        poll: None,
        reactions: vec![Reaction {
            count: 1,
            emoji: EmojiReactionType::Unicode {
                name: "🍕".to_owned(),
            },
            me: false,
            burst_colors: Vec::new(),
            count_details: ReactionCountDetails {
                burst: 0,
                normal: 0,
            },
            me_burst: false,
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
        timestamp: Timestamp::from_micros(123_456_789).unwrap(),
        thread: None,
        tts: false,
        webhook_id: None,
    }
}

pub fn message_update() -> MessageUpdate {
    MessageUpdate(message())
}
