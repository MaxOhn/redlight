use twilight_model::{
    channel::{Channel, ChannelFlags, ChannelType, VideoQualityMode},
    gateway::payload::incoming::ChannelPinsUpdate,
    id::Id,
    util::{ImageHash, Timestamp},
};

pub fn text_channel() -> Channel {
    Channel {
        application_id: None,
        applied_tags: None,
        available_tags: None,
        bitrate: None,
        default_auto_archive_duration: None,
        default_forum_layout: None,
        default_reaction_emoji: None,
        default_sort_order: None,
        default_thread_rate_limit_per_user: None,
        flags: Some(ChannelFlags::PINNED),
        guild_id: Some(Id::new(898)),
        icon: Some(ImageHash::new([1; 16], false)),
        id: Id::new(765),
        invitable: Some(true),
        kind: ChannelType::GuildText,
        last_message_id: Some(Id::new(111)),
        last_pin_timestamp: None,
        managed: None,
        member: None,
        member_count: None,
        message_count: None,
        name: Some("channel_name".to_owned()),
        newly_created: None,
        nsfw: Some(false),
        owner_id: None,
        parent_id: Some(Id::new(222)),
        permission_overwrites: Some(vec![]),
        position: Some(6),
        rate_limit_per_user: None,
        recipients: None,
        rtc_region: None,
        thread_metadata: None,
        topic: Some("channel_topic".to_owned()),
        user_limit: Some(123),
        video_quality_mode: Some(VideoQualityMode::Auto),
    }
}

pub fn channel_pins_update() -> ChannelPinsUpdate {
    ChannelPinsUpdate {
        channel_id: Id::new(765),
        guild_id: Some(Id::new(898)),
        last_pin_timestamp: Some(Timestamp::parse("2022-02-02T02:02:02+00:00").unwrap()),
    }
}
