use twilight_model::{
    channel::{
        message::{
            MessageActivity, MessageActivityType, MessageFlags, MessageType, RoleSubscriptionData,
        },
        Message,
    },
    id::Id,
    util::Timestamp,
};

use super::{member::partial_member, user::user};

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
        mention_channels: vec![], // TODO
        mention_everyone: false,
        mention_roles: vec![Id::new(456), Id::new(567)],
        mentions: vec![], // TODO
        pinned: false,
        reactions: vec![], // TODO
        reference: None,
        referenced_message: None,
        role_subscription_data: Some(RoleSubscriptionData {
            is_renewal: true,
            role_subscription_listing_id: Id::new(100),
            tier_name: "tier name".to_owned(),
            total_months_subscribed: 13,
        }),
        sticker_items: vec![], // TODO
        timestamp: Timestamp::parse("2021-01-01T01:01:01+00:00").unwrap(),
        thread: None,
        tts: false,
        webhook_id: None,
    }
}
