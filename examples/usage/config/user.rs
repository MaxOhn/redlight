use std::time::Duration;

use redlight::{
    config::{Cacheable, ICachedUser},
    rkyv_util::id::{ArchivedId, IdRkyv},
    CachedArchive,
};
use rkyv::{
    option::ArchivedOption, rancor::Source, ser::writer::Buffer, traits::NoUndef, util::Align,
    with::Map, Archive, Archived, Serialize,
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
pub struct CachedUser {
    // twilight's types don't implement rkyv traits so `redlight::rkyv_util`
    // includes a few wrappers out the box. If there's no appropriate wrapper
    // yet, we can just define our own
    #[rkyv(with = Map<CustomImageHashWrap>)]
    avatar: Option<ImageHash>,
    bot: bool,
    #[rkyv(with = IdRkyv)]
    id: Id<UserMarker>,
}

#[derive(Archive, Serialize)]
#[rkyv(remote = ImageHash, archived = ArchivedImageHash)]
struct CustomImageHashWrap {
    // Let's say we don't care whether it's animated or not so we only store
    // the bytes.
    #[rkyv(getter = get_bytes)]
    #[expect(
        unused,
        reason = "only `ArchivedImageHash::bytes` is being used; not `CustomImageHashWrap::bytes`"
    )]
    bytes: [u8; 16],
}

fn get_bytes(image_hash: &ImageHash) -> [u8; 16] {
    image_hash.bytes()
}

impl From<ImageHash> for ArchivedImageHash {
    fn from(image_hash: ImageHash) -> Self {
        Self {
            bytes: image_hash.bytes(),
        }
    }
}

unsafe impl NoUndef for ArchivedImageHash {}

impl<'a> ICachedUser<'a> for CachedUser {
    fn from_user(user: &'a User) -> Self {
        Self {
            avatar: user.avatar,
            bot: user.bot,
            id: user.id,
        }
    }

    fn update_via_partial<E: Source>(
    ) -> Option<fn(&mut CachedArchive<Archived<Self>>, &PartialUser) -> Result<(), E>> {
        Some(|archive, partial| {
            // We can use either `update_archive` or `update_by_deserializing`.
            // Since `update_archive` is much more performant, we'll choose
            // that even though that means we won't be able to update
            // `Option`'s properly.
            archive.update_archive(|sealed| {
                // `munge!` is a great way to access fields of a sealed value
                rkyv::munge::munge!(let ArchivedCachedUser { avatar, mut id, .. } = sealed);

                *id = ArchivedId::from(partial.id);

                // A serialized `Option` cannot be mutated from `Some` to
                // `None` or vice versa so the only updating we're allowed to
                // do here is for `Some` to `Some`.
                if let Some(new_avatar) = partial.avatar {
                    if let Some(mut avatar) = ArchivedOption::as_seal(avatar) {
                        *avatar = ArchivedImageHash::from(new_avatar);
                    }
                }
            });

            Ok(())
        })
    }
}

impl Cacheable for CachedUser {
    type Bytes = [u8; 32];

    fn expire() -> Option<Duration> {
        None
    }

    fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
        // We know exactly how large the serialized container has to be
        // so we can serialize into a perfectly sized array.
        // Just in case the serialized bytes are immediately accessed as
        // archived type, we might as well align the bytes properly.
        // Lastly, none of our fields require scratch space so we don't need
        // rkyv's `to_bytes_*` methods and just use `Buffer` as serializer.
        let mut bytes = Align([0_u8; 32]);
        rkyv::api::serialize_using(self, &mut Buffer::from(&mut *bytes))?;

        Ok(bytes.0)
    }
}
