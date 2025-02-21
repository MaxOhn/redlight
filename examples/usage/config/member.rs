use std::time::Duration;

use redlight::{
    config::{Cacheable, ICachedMember, SerializeMany},
    rkyv_util::id::IdRkyvMap,
    CachedArchive,
};
use rkyv::{rancor::Source, util::AlignedVec, Archive, Archived, Deserialize, Serialize};
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
    ) -> Option<fn(&mut CachedArchive<Archived<Self>>, &PartialMember) -> Result<(), E>> {
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
    ) -> Option<fn(&mut CachedArchive<Archived<Self>>, &MemberUpdate) -> Result<(), E>> {
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

    // This method is optional to be implemented. By default it just forwards
    // to the `serialize_one` method so feel free to skip this.
    // However, when using `CachedArchive<_>::update_by_deserializing`,
    // implementing this method can improve performance by directly
    // serializing into the given bytes instead of first serializing into a
    // new buffer and then copying that buffer.
    fn serialize_into<E: Source, const N: usize>(
        &self,
        bytes: &mut AlignedVec<N>,
    ) -> Result<(), E> {
        // Serializing a `Vec` requires scratch space so our serializer must
        // implement `rkyv::ser::Allocator` but nothing more; perfect for a
        // plain `ArchivedVec`. With `rkyv::api::high::to_bytes_in` we can use
        // the `bytes` we were given as buffer.
        rkyv::api::high::to_bytes_in(self, bytes)?;

        Ok(())
    }

    // This method is required to be implemented.
    fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
        // Since `serialize_into` is implemented, we might as well just
        // leverage that.
        let mut bytes = AlignedVec::<8>::new();
        self.serialize_into(&mut bytes)?;

        Ok(bytes)
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
        next.serialize_into(&mut self.writer)?;

        let mut bytes = AlignedVec::<8>::new();
        bytes.extend_from_slice(self.writer.as_slice());

        Ok(bytes)
    }
}
