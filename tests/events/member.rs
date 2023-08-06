use std::{
    error::Error,
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::Deref,
};

use rkyv::{
    ser::serializers::AlignedSerializer, AlignedVec, Archive, Deserialize, Infallible, Serialize,
};
use twilight_model::{
    gateway::{
        event::Event,
        payload::incoming::{MemberAdd, MemberUpdate, MessageCreate},
    },
    guild::{Member, MemberFlags, PartialMember},
    id::{marker::GuildMarker, Id},
    util::Timestamp,
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedMember, Ignore},
    rkyv_util::flags::BitflagsRkyv,
    CacheError, CachedArchive, RedisCache,
};

use crate::{events::message::message, pool};

use super::user::user;

#[tokio::test]
async fn test_member() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        type Channel<'a> = Ignore;
        type CurrentUser<'a> = Ignore;
        type Emoji<'a> = Ignore;
        type Guild<'a> = Ignore;
        type Integration<'a> = Ignore;
        type Member<'a> = CachedMember;
        type Message<'a> = Ignore;
        type Presence<'a> = Ignore;
        type Role<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedMember {
        #[with(BitflagsRkyv)]
        flags: MemberFlags,
        pending: bool,
    }

    impl<'a> ICachedMember<'a> for CachedMember {
        fn from_member(_: Id<GuildMarker>, member: &'a Member) -> Self {
            Self {
                flags: member.flags,
                pending: member.pending,
            }
        }

        fn on_member_update(
        ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), Box<dyn Error>>>
        {
            Some(|archived, update| {
                archived
                    .update_by_deserializing(
                        |deserialized| deserialized.pending = update.pending,
                        &mut Infallible,
                    )
                    .map_err(Box::from)
            })
        }

        fn update_via_partial(
        ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), Box<dyn Error>>>
        {
            Some(|archived, member| {
                archived.update_archive(|pinned| pinned.get_mut().flags = member.flags.bits());

                Ok(())
            })
        }
    }

    impl Cacheable for CachedMember {
        type Serializer = AlignedSerializer<AlignedVec>;

        fn expire_seconds() -> Option<usize> {
            None
        }
    }

    impl PartialEq<Member> for ArchivedCachedMember {
        fn eq(&self, other: &Member) -> bool {
            self.flags == other.flags.bits() && self.pending == other.pending
        }
    }

    impl Debug for ArchivedCachedMember {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            f.debug_struct("ArchivedCachedMember")
                .field("flags", &self.flags)
                .field("pending", &self.pending)
                .finish()
        }
    }

    let mut expected_member = member();
    let expected_update = member_update();
    let expected_partial = partial_member();

    let guild_id = expected_update.guild_id;

    assert_ne!(expected_member.pending, expected_update.pending);
    assert_ne!(expected_member.flags, expected_partial.flags);

    let cache = RedisCache::<Config>::with_pool(pool());

    let member_create = Event::MemberAdd(Box::new(MemberAdd {
        guild_id,
        member: expected_member.clone(),
    }));

    cache.update(&member_create).await?;

    let member = cache
        .member(guild_id, expected_member.user.id)
        .await?
        .expect("missing member");

    assert_eq!(member.deref(), &expected_member);

    expected_member.pending = expected_update.pending;
    let member_update = Event::MemberUpdate(Box::new(expected_update));

    cache.update(&member_update).await?;

    let member = cache
        .member(guild_id, expected_member.user.id)
        .await?
        .expect("missing member");

    assert_eq!(member.deref(), &expected_member);

    expected_member.flags = expected_partial.flags;

    let message_create = Event::MessageCreate(Box::new(MessageCreate(message())));

    cache.update(&message_create).await?;

    let member = cache
        .member(guild_id, expected_member.user.id)
        .await?
        .expect("missing member");

    assert_eq!(member.deref(), &expected_member);

    let mut iter = cache.iter().guild_members(guild_id).await?;

    let member = iter.next_item().await.expect("missing member")?;
    assert_eq!(member.deref(), &expected_member);

    assert!(iter.next_item().await.is_none());

    Ok(())
}

pub fn member() -> Member {
    Member {
        avatar: None,
        communication_disabled_until: None,
        deaf: false,
        flags: MemberFlags::COMPLETED_ONBOARDING,
        joined_at: Timestamp::parse("2021-01-01T01:01:01+00:00").unwrap(),
        mute: false,
        nick: None,
        pending: true,
        premium_since: None,
        roles: vec![Id::new(123), Id::new(456)],
        user: user(),
    }
}

pub fn member_update() -> MemberUpdate {
    MemberUpdate {
        avatar: None,
        communication_disabled_until: None,
        guild_id: Id::new(111),
        deaf: None,
        joined_at: Timestamp::parse("2021-01-01T01:01:01+00:00").unwrap(),
        mute: None,
        nick: None,
        pending: false,
        premium_since: None,
        roles: vec![Id::new(123), Id::new(456)],
        user: user(),
    }
}

pub fn partial_member() -> PartialMember {
    PartialMember {
        avatar: None,
        communication_disabled_until: None,
        deaf: false,
        flags: MemberFlags::empty(),
        joined_at: Timestamp::parse("2021-01-01T01:01:01+00:00").unwrap(),
        mute: false,
        nick: None,
        permissions: None,
        premium_since: None,
        roles: vec![Id::new(123), Id::new(456)],
        user: Some(user()),
    }
}
