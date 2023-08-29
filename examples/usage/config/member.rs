use std::{error::Error, time::Duration};

use redlight::{
    config::{Cacheable, ICachedMember},
    rkyv_util::id::IdRkyv,
    CachedArchive,
};
use rkyv::{
    ser::serializers::AllocSerializer, with::Map, Archive, Deserialize, Infallible, Serialize,
};
use twilight_model::{
    gateway::payload::incoming::MemberUpdate,
    guild::{Member, PartialMember},
    id::{
        marker::{GuildMarker, RoleMarker},
        Id,
    },
};

// We're only interested in the member's nickname and roles
// so we don't need anything else.
#[derive(Archive, Serialize, Deserialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub struct CachedMember {
    nick: Option<String>,
    #[with(Map<IdRkyv>)]
    roles: Vec<Id<RoleMarker>>,
}

impl<'a> ICachedMember<'a> for CachedMember {
    fn from_member(_guild_id: Id<GuildMarker>, member: &'a Member) -> Self {
        Self {
            nick: member.nick.clone(),
            roles: member.roles.clone(),
        }
    }

    fn update_via_partial(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), Box<dyn Error>>> {
        Some(|archive, partial| {
            // We can use either `update_archive` or `update_by_deserializing`.
            // Our archived fields will be of variable length so we cannot update
            // a pinned archive. Hence, we're forced to use the less performant
            // `update_by_deserializing` method and require a `Deserialize` impl
            // on our `CachedMember`.
            archive.update_by_deserializing(
                |deserialized| {
                    deserialized.nick = partial.nick.clone();
                    deserialized.roles = partial.roles.clone();
                },
                &mut Infallible,
            )
        })
    }

    fn on_member_update(
    ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), Box<dyn Error>>> {
        Some(|archive, partial| {
            archive.update_by_deserializing(
                |deserialized| {
                    deserialized.nick = partial.nick.clone();
                    deserialized.roles = partial.roles.clone();
                },
                &mut Infallible,
            )
        })
    }
}

impl Cacheable for CachedMember {
    // Serializing a `Vec` requires scratch space, `AllocSerializer` being a good general-purpose default.
    // rkyv's `ScratchTracker` can help to find a reasonable scratch size.
    // In our case, each element will require a scratch size of 8 bytes so 64 should
    // suffice for roles without causing too many re-allocations.
    type Serializer = AllocSerializer<64>;

    fn expire() -> Option<Duration> {
        None
    }
}
