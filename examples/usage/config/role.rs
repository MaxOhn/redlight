use std::time::Duration;

use redlight::{
    config::{Cacheable, ICachedRole},
    rkyv_util::util::BitflagsRkyv,
};
use rkyv::{ser::serializers::AlignedSerializer, with::RefAsBox, AlignedVec, Archive, Serialize};
use twilight_model::guild::{Permissions, Role};

// We're only interested in the role's name and permissions
// so we don't need anything else.
#[derive(Archive, Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub struct CachedRole<'a> {
    #[with(RefAsBox)]
    name: &'a str,
    // twilight's types don't implement rkyv traits
    // so we use wrappers from `redlight::rkyv_util`
    #[with(BitflagsRkyv)]
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
    // We don't have any fields that require scratch space so we don't need an `AllocSerializer`
    // but we do have a str of which we don't know the length so our byte buffer needs to be flexible.
    type Serializer = AlignedSerializer<AlignedVec>;

    fn expire() -> Option<Duration> {
        None
    }
}
