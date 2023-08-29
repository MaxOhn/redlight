use rkyv::{
    out_field,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Fallible,
};
use twilight_model::util::ImageHash;

/// Used to archive [`ImageHash`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::util::ImageHashRkyv;
/// use twilight_model::util::ImageHash;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[with(ImageHashRkyv)]
///     avatar: ImageHash,
/// }
/// ```
pub struct ImageHashRkyv;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
/// An archived [`ImageHash`].
pub struct ArchivedImageHash {
    animated: Archived<bool>,
    bytes: Archived<[u8; 16]>,
}

impl ArchivedImageHash {
    /// Efficiently packed bytes of the hash.
    pub const fn bytes(self) -> [u8; 16] {
        self.bytes
    }

    /// Whether the hash is for an animated image.
    pub const fn is_animated(self) -> bool {
        self.animated
    }
}

impl From<ArchivedImageHash> for ImageHash {
    fn from(archived: ArchivedImageHash) -> Self {
        ImageHash::new(archived.bytes, archived.animated)
    }
}

impl From<ImageHash> for ArchivedImageHash {
    fn from(hash: ImageHash) -> Self {
        Self {
            animated: hash.is_animated(),
            bytes: hash.bytes(),
        }
    }
}

impl ArchiveWith<ImageHash> for ImageHashRkyv {
    type Archived = ArchivedImageHash;
    type Resolver = ();

    unsafe fn resolve_with(
        hash: &ImageHash,
        pos: usize,
        _: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.animated);

        #[allow(clippy::unit_arg)]
        Archive::resolve(&hash.is_animated(), pos + fp, (), fo);

        let (fp, fo) = out_field!(out.bytes);
        Archive::resolve(&hash.bytes(), pos + fp, [(); 16], fo);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<ImageHash, S> for ImageHashRkyv {
    fn serialize_with(_: &ImageHash, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedImageHash, ImageHash, D> for ImageHashRkyv {
    fn deserialize_with(
        archived: &ArchivedImageHash,
        deserializer: &mut D,
    ) -> Result<ImageHash, <D as Fallible>::Error> {
        archived.deserialize(deserializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<ImageHash, D> for ArchivedImageHash {
    fn deserialize(&self, _: &mut D) -> Result<ImageHash, <D as Fallible>::Error> {
        Ok(ImageHash::new(self.bytes, self.animated))
    }
}

#[cfg(feature = "validation")]
const _: () =
    {
        use std::ptr::addr_of;

        use rkyv::{
            bytecheck::{ErrorBox, StructCheckError},
            CheckBytes,
        };

        impl<C: ?Sized> CheckBytes<C> for ArchivedImageHash {
            type Error = StructCheckError;

            unsafe fn check_bytes<'a>(
                value: *const Self,
                context: &mut C,
            ) -> Result<&'a Self, Self::Error> {
                <bool as CheckBytes<C>>::check_bytes(addr_of!((*value).animated), context)
                    .map_err(|e| StructCheckError {
                        field_name: "animated",
                        inner: ErrorBox::new(e),
                    })?;

                <[u8; 16] as CheckBytes<C>>::check_bytes(addr_of!((*value).bytes), context)
                    .map_err(|e| StructCheckError {
                        field_name: "bytes",
                        inner: ErrorBox::new(e),
                    })?;

                Ok(&*value)
            }
        }
    };

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_image_hash() {
        type Wrapper = With<ImageHash, ImageHashRkyv>;

        let image_hash = ImageHash::new([1; 16], false);
        let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&image_hash)).unwrap();

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

        let deserialized: ImageHash = archived.deserialize(&mut Infallible).unwrap();

        assert_eq!(image_hash, deserialized);
    }
}
