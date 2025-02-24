use rkyv::{
    munge::munge,
    niche::niching::{Bool, Niching},
    Archive, Deserialize, Place, Serialize,
};
use twilight_model::util::ImageHash;

/// Used to archive and niche an [`ImageHash`].
///
/// In case of an [`Option<ImageHash>`], instead of using
/// [`Map<ImageHashRkyv>`] you should be using
/// [`MapNiche<ImageHashRkyv, ImageHashRkyv>`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::image_hash::ImageHashRkyv;
/// use rkyv::with::MapNiche;
/// use twilight_model::util::ImageHash;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = ImageHashRkyv)]
///     image_hash: ImageHash,
///     #[rkyv(with = MapNiche<ImageHashRkyv, ImageHashRkyv>)]
///     option: Option<ImageHash>,
/// }
/// ```
///
/// [`Map<ImageHashRkyv>`]: rkyv::with::Map
/// [`MapNiche<ImageHashRkyv, ImageHashRkyv>`]: rkyv::with::MapNiche
#[derive(Archive, Serialize, Deserialize)]
#[rkyv(
    remote = ImageHash,
    archived = ArchivedImageHash,
    resolver = ImageHashResolver,
    derive(Copy, Clone, PartialEq, Eq),
)]
pub struct ImageHashRkyv {
    #[rkyv(getter = get_animated)]
    pub animated: bool,
    #[rkyv(getter = get_bytes)]
    pub bytes: [u8; 16],
}

fn get_animated(image_hash: &ImageHash) -> bool {
    image_hash.is_animated()
}

fn get_bytes(image_hash: &ImageHash) -> [u8; 16] {
    image_hash.bytes()
}

macro_rules! impl_from {
    ($ty:ty) => {
        impl From<$ty> for ImageHash {
            fn from(image_hash: $ty) -> Self {
                ImageHash::new(image_hash.bytes, image_hash.animated)
            }
        }
    };
}

impl_from!(ImageHashRkyv);
impl_from!(ArchivedImageHash);

impl PartialEq<ImageHash> for ArchivedImageHash {
    fn eq(&self, other: &ImageHash) -> bool {
        self.bytes == other.bytes() && self.animated == other.is_animated()
    }
}

impl PartialEq<ArchivedImageHash> for ImageHash {
    fn eq(&self, other: &ArchivedImageHash) -> bool {
        other.eq(self)
    }
}

impl Niching<ArchivedImageHash> for ImageHashRkyv {
    unsafe fn is_niched(niched: *const ArchivedImageHash) -> bool {
        unsafe { <Bool as Niching<bool>>::is_niched(&raw const (*niched).animated) }
    }

    fn resolve_niched(out: Place<ArchivedImageHash>) {
        munge!(let ArchivedImageHash { animated, .. } = out);

        <Bool as Niching<bool>>::resolve_niched(animated)
    }
}

impl ArchivedImageHash {
    /// Convert an [`ArchivedImageHash`] to an [`ImageHash`].
    pub const fn to_native(self) -> ImageHash {
        ImageHash::new(self.bytes, self.animated)
    }

    /// Convert an [`ImageHash`] to an [`ArchivedImageHash`].
    pub const fn from_native(image_hash: ImageHash) -> Self {
        Self {
            animated: image_hash.is_animated(),
            bytes: image_hash.bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{
        niche::niched_option::NichedOption,
        rancor::Error,
        with::{MapNiche, With},
    };

    use super::*;

    #[test]
    fn test_rkyv_image_hash() -> Result<(), Error> {
        let options = [
            Some(ImageHash::new([5; 16], true)),
            Some(ImageHash::new([8; 16], false)),
            None,
        ];

        for opt in options {
            let bytes = rkyv::to_bytes(With::<_, MapNiche<ImageHashRkyv, ImageHashRkyv>>::cast(
                &opt,
            ))?;

            #[cfg(not(feature = "bytecheck"))]
            let archived: &NichedOption<ArchivedImageHash, ImageHashRkyv> =
                unsafe { rkyv::access_unchecked(&bytes) };

            #[cfg(feature = "bytecheck")]
            let archived: &NichedOption<ArchivedImageHash, ImageHashRkyv> = rkyv::access(&bytes)?;

            let deserialized: Option<ImageHash> = rkyv::deserialize(With::<
                _,
                MapNiche<ImageHashRkyv, ImageHashRkyv>,
            >::cast(archived))?;

            assert_eq!(opt, deserialized);
        }

        Ok(())
    }
}
