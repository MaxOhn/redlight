use std::time::Duration;

use redlight::{
    config::{Cacheable, ICachedRole},
    rkyv_util::flags::BitflagsRkyv,
};
use rkyv::{rancor::Source, util::AlignedVec, with::InlineAsBox, Archive, Serialize};
use twilight_model::guild::{Permissions, Role};

// We're only interested in the role's name and permissions
// so we don't need anything else.
#[derive(Archive, Serialize)]
pub struct CachedRole<'a> {
    #[rkyv(with = InlineAsBox)]
    name: &'a str,
    // twilight's types don't implement rkyv traits
    // so we use wrappers from `redlight::rkyv_util`
    #[rkyv(with = BitflagsRkyv)]
    permissions: Permissions,
}

impl<'a> ICachedRole<'a> for CachedRole<'a> {
    fn from_role(role: &'a Role) -> Self {
        Self {
            name: &role.name,
            permissions: role.permissions,
        }
    }
}

impl Cacheable for CachedRole<'_> {
    type Bytes = AlignedVec<8>;

    fn expire() -> Option<Duration> {
        None
    }

    fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
        // The `name` field is of variable length so we need to serialize into
        // a resizable buffer. Furthermore, no field requires scratch space
        // so our serializer only needs to implement `rkyv::ser::Writer` but
        // not `rkyv::ser::Allocator` or `rkyv::ser::Sharing`.
        // Hence, we don't need `rkyv::ser::Serializer` but can use
        // `AlignedVec` directly.
        let mut serializer = AlignedVec::<8>::new();
        rkyv::api::serialize_using(self, &mut serializer)?;

        Ok(serializer)
    }
}
