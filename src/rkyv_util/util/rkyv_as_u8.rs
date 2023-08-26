use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Fallible,
};

/// Used to archive any `T` for which `u8: From<T>` holds such as [`IntegrationExpireBehavior`](twilight_model::guild::IntegrationExpireBehavior) or [`StickerType`](twilight_model::channel::message::sticker::StickerType).
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use rkyv::with::Map;
/// use twilight_model::channel::message::sticker::StickerType;
/// use twilight_model::guild::IntegrationExpireBehavior;
/// use twilight_redis::rkyv_util::util::RkyvAsU8;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[with(RkyvAsU8)]
///     expire_behavior: IntegrationExpireBehavior,
///     #[with(Map<RkyvAsU8>)]
///     sticker_kind: Option<StickerType>,
/// }
/// ```
pub struct RkyvAsU8;

impl<T> ArchiveWith<T> for RkyvAsU8
where
    T: Copy,
    u8: From<T>,
{
    type Archived = Archived<u8>;
    type Resolver = ();

    unsafe fn resolve_with(
        field: &T,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        u8::from(*field).resolve(pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized, T> SerializeWith<T, S> for RkyvAsU8
where
    T: Copy,
    u8: From<T>,
{
    fn serialize_with(_: &T, _: &mut S) -> Result<(), <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized, T> DeserializeWith<Archived<u8>, T, D> for RkyvAsU8
where
    T: From<u8>,
{
    fn deserialize_with(archived: &Archived<u8>, _: &mut D) -> Result<T, <D as Fallible>::Error> {
        Ok(T::from(*archived))
    }
}
