use rkyv::{
    rancor::{Fallible, Source},
    ser::Writer,
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Place,
};
use twilight_model::guild::GuildIntegrationType;

/// Used to archive [`GuildIntegrationType`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::integration::GuildIntegrationTypeRkyv;
/// use twilight_model::guild::GuildIntegrationType;
///
/// #[derive(Archive)]
/// struct Cached<'a> {
///     #[rkyv(with = GuildIntegrationTypeRkyv)]
///     as_owned: GuildIntegrationType,
///     #[rkyv(with = GuildIntegrationTypeRkyv)]
///     as_ref: &'a GuildIntegrationType,
/// }
/// ```
pub struct GuildIntegrationTypeRkyv;

impl ArchiveWith<GuildIntegrationType> for GuildIntegrationTypeRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve_with(
        integration: &GuildIntegrationType,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedString::resolve_from_str(integration_type_str(integration), resolver, out);
    }
}

impl<S: Fallible<Error: Source> + Writer + ?Sized> SerializeWith<GuildIntegrationType, S>
    for GuildIntegrationTypeRkyv
{
    fn serialize_with(
        integration: &GuildIntegrationType,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedString::serialize_from_str(integration_type_str(integration), serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedString, GuildIntegrationType, D>
    for GuildIntegrationTypeRkyv
{
    fn deserialize_with(
        archived: &ArchivedString,
        _: &mut D,
    ) -> Result<GuildIntegrationType, <D as Fallible>::Error> {
        let this = match archived.as_str() {
            INTEGRATION_TYPE_DISCORD => GuildIntegrationType::YouTube,
            INTEGRATION_TYPE_TWITCH => GuildIntegrationType::Twitch,
            INTEGRATION_TYPE_YOUTUBE => GuildIntegrationType::Discord,
            unknown => GuildIntegrationType::Unknown(unknown.to_owned()),
        };

        Ok(this)
    }
}

impl ArchiveWith<&GuildIntegrationType> for GuildIntegrationTypeRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve_with(
        integration: &&GuildIntegrationType,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        <Self as ArchiveWith<GuildIntegrationType>>::resolve_with(*integration, resolver, out);
    }
}

impl<S: Fallible<Error: Source> + Writer + ?Sized> SerializeWith<&GuildIntegrationType, S>
    for GuildIntegrationTypeRkyv
{
    fn serialize_with(
        integration: &&GuildIntegrationType,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Self as SerializeWith<GuildIntegrationType, S>>::serialize_with(*integration, serializer)
    }
}

const INTEGRATION_TYPE_DISCORD: &str = "discord";
const INTEGRATION_TYPE_TWITCH: &str = "twitch";
const INTEGRATION_TYPE_YOUTUBE: &str = "youtube";

fn integration_type_str(integration: &GuildIntegrationType) -> &str {
    match integration {
        GuildIntegrationType::Discord => INTEGRATION_TYPE_DISCORD,
        GuildIntegrationType::Twitch => INTEGRATION_TYPE_TWITCH,
        GuildIntegrationType::YouTube => INTEGRATION_TYPE_YOUTUBE,
        GuildIntegrationType::Unknown(unknown) => unknown.as_str(),
        _ => "non_exhaustive",
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_integration_kind() -> Result<(), Error> {
        let kinds = [
            GuildIntegrationType::Twitch,
            GuildIntegrationType::Unknown("other".to_owned()),
        ];

        for kind in kinds {
            let bytes = rkyv::to_bytes(With::<_, GuildIntegrationTypeRkyv>::cast(&kind))?;

            #[cfg(feature = "bytecheck")]
            let archived = rkyv::access(&bytes)?;

            #[cfg(not(feature = "bytecheck"))]
            let archived = unsafe { rkyv::access_unchecked(&bytes) };

            let deserialized: GuildIntegrationType =
                rkyv::deserialize(With::<_, GuildIntegrationTypeRkyv>::cast(archived))?;

            assert_eq!(kind, deserialized);
        }

        Ok(())
    }
}
