use rkyv::{
    rancor::Fallible,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Place,
};
use twilight_model::channel::stage_instance::PrivacyLevel;

/// Used to archive [`PrivacyLevel`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::stage_instance::PrivacyLevelRkyv;
/// use twilight_model::channel::stage_instance::PrivacyLevel;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = PrivacyLevelRkyv)]
///     privacy_level: PrivacyLevel,
/// }
/// ```
pub struct PrivacyLevelRkyv;

impl ArchiveWith<PrivacyLevel> for PrivacyLevelRkyv {
    type Archived = Archived<u8>;
    type Resolver = ();

    fn resolve_with(level: &PrivacyLevel, resolver: Self::Resolver, out: Place<Self::Archived>) {
        (*level as u8).resolve(resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<PrivacyLevel, S> for PrivacyLevelRkyv {
    fn serialize_with(_: &PrivacyLevel, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<u8>, PrivacyLevel, D> for PrivacyLevelRkyv {
    fn deserialize_with(_: &Archived<u8>, _: &mut D) -> Result<PrivacyLevel, D::Error> {
        Ok(PrivacyLevel::GuildOnly) // currently the only variant
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_privacy_level() -> Result<(), Error> {
        let level = PrivacyLevel::GuildOnly;
        let bytes = rkyv::to_bytes(With::<_, PrivacyLevelRkyv>::cast(&level))?;

        #[cfg(feature = "bytecheck")]
        let archived: &Archived<u8> = rkyv::access(&bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &Archived<u8> = unsafe { rkyv::access_unchecked(&bytes) };

        let deserialized: PrivacyLevel =
            rkyv::deserialize(With::<_, PrivacyLevelRkyv>::cast(archived))?;

        assert_eq!(level, deserialized);

        Ok(())
    }
}
