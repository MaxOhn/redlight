use std::time::Duration;

use redlight::{
    config::{Cacheable, ICachedMember, SerializeMany},
    rkyv_util::id::IdRkyvMap,
    CachedArchive,
};
use rkyv::{rancor::Source, util::AlignedVec, Archive, Deserialize, Serialize};
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
pub struct CachedMember {
    nick: Option<String>,
    #[rkyv(with = IdRkyvMap)] // More efficient than `Map<IdRkyv>`
    roles: Vec<Id<RoleMarker>>,
}

impl<'a> ICachedMember<'a> for CachedMember {
    fn from_member(_guild_id: Id<GuildMarker>, member: &'a Member) -> Self {
        Self {
            nick: member.nick.clone(),
            roles: member.roles.clone(),
        }
    }

    fn update_via_partial<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialMember) -> Result<(), E>> {
        Some(|archive, partial| {
            // We can use either `update_archive` or `update_by_deserializing`.
            // Our archived fields will be of variable length so we cannot update
            // a sealed archive. Hence, we're forced to use the less performant
            // `update_by_deserializing` method and require a `Deserialize` impl
            // on our `CachedMember`.
            archive
                .update_by_deserializing(
                    |deserialized| {
                        deserialized.nick = partial.nick.clone();
                        deserialized.roles.clone_from(&partial.roles);
                    },
                    &mut (),
                )
                .map_err(Source::new)
        })
    }

    fn on_member_update<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Self>, &MemberUpdate) -> Result<(), E>> {
        Some(|archive, partial| {
            archive
                .update_by_deserializing(
                    |deserialized| {
                        deserialized.nick = partial.nick.clone();
                        deserialized.roles = partial.roles.clone();
                    },
                    &mut (),
                )
                .map_err(Source::new)
        })
    }
}

impl Cacheable for CachedMember {
    type Bytes = AlignedVec<8>;

    fn expire() -> Option<Duration> {
        None
    }

    fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
        // Serializing a `Vec` requires scratch space so our serializer must
        // implement `rkyv::ser::Allocator`. We could use `rkyv::to_bytes` but
        // since our fields don't require an alignment of 16, we can use
        // `rkyv::api::high::to_bytes_in` instead to specify our own alignment.
        rkyv::api::high::to_bytes_in(self, AlignedVec::<8>::new())
    }

    // Let's be fancy and implement this optional method to slightly improve
    // performance.
    fn serialize_many() -> impl SerializeMany<Self> {
        SerializeWithRecycle::default()
    }
}

#[derive(Default)]
/// Always serializes into the same byte buffer to avoid reallocations.
struct SerializeWithRecycle {
    writer: AlignedVec<8>,
}

impl SerializeMany<CachedMember> for SerializeWithRecycle {
    type Bytes = AlignedVec<8>;

    fn serialize_next<E: Source>(&mut self, next: &CachedMember) -> Result<Self::Bytes, E> {
        self.writer.clear();
        rkyv::api::high::to_bytes_in(next, &mut self.writer)?;

        let mut bytes = AlignedVec::<8>::new();
        bytes.extend_from_slice(self.writer.as_slice());

        Ok(bytes)
    }
}
