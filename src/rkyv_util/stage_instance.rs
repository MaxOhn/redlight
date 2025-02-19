use rkyv::{rancor::Fallible, Archive, Deserialize, Serialize};
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
#[derive(Archive, Serialize, Deserialize)]
#[rkyv(
    remote = PrivacyLevel,
    archived = ArchivedPrivacyLevel,
    resolver = PrivacyLevelResolver,
    derive(Copy, Clone, Debug, PartialEq, Eq),
)]
#[repr(u8)]
pub enum PrivacyLevelRkyv {
    GuildOnly,
    #[rkyv(other)]
    Unknown,
}

macro_rules! impl_from {
    ($ty:ident) => {
        impl From<$ty> for PrivacyLevel {
            fn from(level: $ty) -> Self {
                match level {
                    $ty::GuildOnly => PrivacyLevel::GuildOnly,
                    $ty::Unknown => PrivacyLevel::GuildOnly,
                }
            }
        }
    };
}

impl_from!(PrivacyLevelRkyv);
impl_from!(ArchivedPrivacyLevel);

impl<D: Fallible + ?Sized> Deserialize<PrivacyLevel, D> for ArchivedPrivacyLevel {
    fn deserialize(&self, _: &mut D) -> Result<PrivacyLevel, <D as Fallible>::Error> {
        Ok(PrivacyLevel::from(*self))
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
        let archived: &ArchivedPrivacyLevel = rkyv::access(&bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &ArchivedPrivacyLevel = unsafe { rkyv::access_unchecked(&bytes) };

        let deserialized: PrivacyLevel =
            rkyv::deserialize(With::<_, PrivacyLevelRkyv>::cast(archived))?;

        assert_eq!(level, deserialized);

        Ok(())
    }
}
