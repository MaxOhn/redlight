use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Fallible,
};
use twilight_model::channel::stage_instance::PrivacyLevel;

/// Used to archive [`PrivacyLevel`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use twilight_model::channel::stage_instance::PrivacyLevel;
/// use twilight_redis::rkyv_util::stage_instance::PrivacyLevelRkyv;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[with(PrivacyLevelRkyv)]
///     privacy_level: PrivacyLevel,
/// }
/// ```
pub struct PrivacyLevelRkyv;

impl ArchiveWith<PrivacyLevel> for PrivacyLevelRkyv {
    type Archived = Archived<u8>;
    type Resolver = ();

    unsafe fn resolve_with(
        level: &PrivacyLevel,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        (*level as u8).resolve(pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<PrivacyLevel, S> for PrivacyLevelRkyv {
    fn serialize_with(
        _: &PrivacyLevel,
        _: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<u8>, PrivacyLevel, D> for PrivacyLevelRkyv {
    fn deserialize_with(
        _: &Archived<u8>,
        _: &mut D,
    ) -> Result<PrivacyLevel, <D as Fallible>::Error> {
        Ok(PrivacyLevel::GuildOnly) // currently the only variant
    }
}