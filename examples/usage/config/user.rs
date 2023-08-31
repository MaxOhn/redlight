use std::time::Duration;

use redlight::{
    config::{Cacheable, ICachedUser},
    error::BoxedError,
    rkyv_util::{
        id::{ArchivedId, IdRkyv},
        util::{ArchivedImageHash, ImageHashRkyv},
    },
    CachedArchive,
};
use rkyv::{
    option::ArchivedOption, ser::serializers::BufferSerializer, with::Map, AlignedBytes, Archive,
    Serialize,
};
use twilight_model::{
    gateway::payload::incoming::invite_create::PartialUser,
    id::{marker::UserMarker, Id},
    user::User,
    util::ImageHash,
};

// We're only interested in the user's avatar, bot status, and id
// so we don't need anything else.
#[derive(Archive, Serialize)]
#[cfg_attr(feature = "validation", archive(check_bytes))]
pub struct CachedUser {
    // twilight's types don't implement rkyv traits
    // so we use wrappers from `redlight::rkyv_util`
    #[with(Map<ImageHashRkyv>)]
    avatar: Option<ImageHash>,
    bot: bool,
    #[with(IdRkyv)]
    id: Id<UserMarker>,
}

impl<'a> ICachedUser<'a> for CachedUser {
    fn from_user(user: &'a User) -> Self {
        Self {
            avatar: user.avatar,
            bot: user.bot,
            id: user.id,
        }
    }

    fn update_via_partial(
    ) -> Option<fn(&mut CachedArchive<Self>, &PartialUser) -> Result<(), BoxedError>> {
        Some(|archive, partial| {
            // We can use either `update_archive` or `update_by_deserializing`.
            // Since we don't have any super complex types and `update_archive`
            // is much more performant, we choose that.
            archive.update_archive(|mut pinned| {
                pinned.avatar = partial
                    .avatar
                    .map(ArchivedImageHash::from)
                    .map_or(ArchivedOption::None, ArchivedOption::Some);
                pinned.id = ArchivedId::from(partial.id);
            });

            Ok(())
        })
    }
}

impl Cacheable for CachedUser {
    // We know exactly how large the serialized container has to be
    // so we can get away with a `BufferSerializer`
    type Serializer = BufferSerializer<AlignedBytes<32>>;

    fn expire() -> Option<Duration> {
        None
    }
}
