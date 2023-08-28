use rkyv::{
    ser::Serializer,
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Fallible,
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
///     #[with(GuildIntegrationTypeRkyv)]
///     as_owned: GuildIntegrationType,
///     #[with(GuildIntegrationTypeRkyv)]
///     as_ref: &'a GuildIntegrationType,
/// }
/// ```
pub struct GuildIntegrationTypeRkyv;

impl ArchiveWith<GuildIntegrationType> for GuildIntegrationTypeRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    unsafe fn resolve_with(
        integration: &GuildIntegrationType,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedString::resolve_from_str(integration_type_str(integration), pos, resolver, out);
    }
}

impl<S: Fallible + Serializer + ?Sized> SerializeWith<GuildIntegrationType, S>
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

    unsafe fn resolve_with(
        integration: &&GuildIntegrationType,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        <Self as ArchiveWith<GuildIntegrationType>>::resolve_with(*integration, pos, resolver, out);
    }
}

impl<S: Fallible + Serializer + ?Sized> SerializeWith<&GuildIntegrationType, S>
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
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_integration_kind() {
        type Wrapper = With<GuildIntegrationType, GuildIntegrationTypeRkyv>;

        let kinds = [
            GuildIntegrationType::Twitch,
            GuildIntegrationType::Unknown("other".to_owned()),
        ];

        for kind in kinds {
            let bytes = rkyv::to_bytes::<_, 16>(Wrapper::cast(&kind)).unwrap();
            let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };
            let deserialized: GuildIntegrationType =
                GuildIntegrationTypeRkyv::deserialize_with(archived, &mut Infallible).unwrap();

            assert_eq!(kind, deserialized);
        }
    }
}
