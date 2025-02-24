use rkyv::{
    rancor::Fallible,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Place,
};

/// Used to archive any `T` for which `u8: From<T>` holds such as
/// [`IntegrationExpireBehavior`], [`PermissionOverwriteType`] or
/// [`StickerType`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::util::RkyvAsU8;
/// use rkyv::with::Map;
/// use twilight_model::{
///     channel::message::sticker::StickerType, guild::IntegrationExpireBehavior,
/// };
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = RkyvAsU8)]
///     expire_behavior: IntegrationExpireBehavior,
///     #[rkyv(with = Map<RkyvAsU8>)]
///     sticker_kind: Option<StickerType>,
/// }
/// ```
///
/// [`IntegrationExpireBehavior`]: twilight_model::guild::IntegrationExpireBehavior
/// [`PermissionOverwriteType`]: twilight_model::channel::permission_overwrite::PermissionOverwriteType
/// [`StickerType`]: twilight_model::channel::message::sticker::StickerType
pub struct RkyvAsU8;

impl<T> ArchiveWith<T> for RkyvAsU8
where
    T: Copy,
    u8: From<T>,
{
    type Archived = Archived<u8>;
    type Resolver = ();

    fn resolve_with(field: &T, resolver: Self::Resolver, out: Place<Self::Archived>) {
        u8::from(*field).resolve(resolver, out);
    }
}

impl<S, T> SerializeWith<T, S> for RkyvAsU8
where
    T: Copy,
    u8: From<T>,
    S: Fallible + ?Sized,
{
    fn serialize_with(_: &T, _: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized, T> DeserializeWith<Archived<u8>, T, D> for RkyvAsU8
where
    T: From<u8>,
{
    fn deserialize_with(archived: &Archived<u8>, _: &mut D) -> Result<T, D::Error> {
        Ok(T::from(*archived))
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};
    use twilight_model::guild::IntegrationExpireBehavior;

    use super::*;

    #[test]
    fn test_rkyv_as_u8() -> Result<(), Error> {
        let behaviors = [
            IntegrationExpireBehavior::RemoveRole,
            IntegrationExpireBehavior::Unknown(u8::MAX),
        ];

        for behavior in behaviors {
            let bytes = rkyv::to_bytes(With::<_, RkyvAsU8>::cast(&behavior))?;

            #[cfg(feature = "bytecheck")]
            let archived: &Archived<u8> = rkyv::access(&bytes)?;

            #[cfg(not(feature = "bytecheck"))]
            let archived: &Archived<u8> = unsafe { rkyv::access_unchecked(&bytes) };

            let deserialized: IntegrationExpireBehavior =
                rkyv::deserialize(With::<_, RkyvAsU8>::cast(archived))?;

            assert_eq!(behavior, deserialized);
        }

        Ok(())
    }
}
