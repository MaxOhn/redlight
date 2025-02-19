use rkyv::{
    rancor::{Fallible, Source},
    ser::Writer,
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Place,
};
use twilight_model::guild::GuildFeature;

/// Used to archive [`GuildFeature`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::guild::GuildFeatureRkyv;
/// use twilight_model::guild::GuildFeature;
///
/// #[derive(Archive)]
/// struct Cached<'a> {
///     #[rkyv(with = GuildFeatureRkyv)]
///     as_owned: GuildFeature,
///     #[rkyv(with = GuildFeatureRkyv)]
///     as_ref: &'a GuildFeature,
/// }
/// ```
pub struct GuildFeatureRkyv;

impl ArchiveWith<GuildFeature> for GuildFeatureRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve_with(feature: &GuildFeature, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(guild_feature_to_str(feature), resolver, out);
    }
}

impl<S: Fallible<Error: Source> + Writer + ?Sized> SerializeWith<GuildFeature, S>
    for GuildFeatureRkyv
{
    fn serialize_with(
        feature: &GuildFeature,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedString::serialize_from_str(guild_feature_to_str(feature), serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedString, GuildFeature, D> for GuildFeatureRkyv {
    fn deserialize_with(
        archived: &ArchivedString,
        _: &mut D,
    ) -> Result<GuildFeature, <D as Fallible>::Error> {
        Ok(guild_feature_from_str(archived.as_str()))
    }
}

impl ArchiveWith<&GuildFeature> for GuildFeatureRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve_with(feature: &&GuildFeature, resolver: Self::Resolver, out: Place<Self::Archived>) {
        <Self as ArchiveWith<GuildFeature>>::resolve_with(feature, resolver, out);
    }
}

impl<S: Fallible<Error: Source> + Writer + ?Sized> SerializeWith<&GuildFeature, S>
    for GuildFeatureRkyv
{
    fn serialize_with(
        feature: &&GuildFeature,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Self as SerializeWith<GuildFeature, S>>::serialize_with(feature, serializer)
    }
}

macro_rules! impl_guild_features {
    ( $( $variant:ident: $str:literal, )* ) => {
        fn guild_feature_from_str(str: &str) -> GuildFeature {
            match str {
                $( $str => GuildFeature::$variant, )*
                unknown => GuildFeature::Unknown(unknown.to_owned()),
            }
        }

        fn guild_feature_to_str(feature: &GuildFeature) -> &str {
            match feature {
                $( GuildFeature::$variant => $str, )*
                GuildFeature::Unknown(unknown) => unknown.as_str(),
                _ => "non_exhaustive",
            }
        }
    }
}

impl_guild_features! {
    AnimatedBanner: "ANIMATED_BANNER",
    AnimatedIcon: "ANIMATED_ICON",
    AutoModeration: "AUTO_MODERATION",
    Banner: "BANNER",
    Community: "COMMUNITY",
    CreatorMonetizableProvisional: "CREATOR_MONETIZABLE_PROVISIONAL",
    CreatorStorePage: "CREATOR_STORE_PAGE",
    DeveloperSupportServer: "DEVELOPER_SUPPORT_SERVER",
    Discoverable: "DISCOVERABLE",
    Featurable: "FEATURABLE",
    InvitesDisabled: "INVITES_DISABLED",
    InviteSplash: "INVITE_SPLASH",
    MemberVerificationGateEnabled: "MEMBER_VERIFICATION_GATE_ENABLED",
    MoreStickers: "MORE_STICKERS",
    News: "NEWS",
    Partnered: "PARTNERED",
    PreviewEnabled: "PREVIEW_ENABLED",
    PrivateThreads: "PRIVATE_THREADS",
    RaidAlertsDisabled: "RAID_ALERTS_DISABLED",
    RoleIcons: "ROLE_ICONS",
    RoleSubscriptionsAvailableForPurchase: "ROLE_SUBSCRIPTIONS_AVAILABLE_FOR_PURCHASE",
    RoleSubscriptionsEnabled: "ROLE_SUBSCRIPTIONS_ENABLED",
    TicketedEventsEnabled: "TICKETED_EVENTS_ENABLED",
    VanityUrl: "VANITY_URL",
    Verified: "VERIFIED",
    VipRegions: "VIP_REGIONS",
    WelcomeScreenEnabled: "WELCOME_SCREEN_ENABLED",
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_guild_feature() -> Result<(), Error> {
        let features = [
            GuildFeature::Banner,
            GuildFeature::Unknown("other".to_owned()),
        ];

        for feature in features {
            let bytes = rkyv::to_bytes(With::<_, GuildFeatureRkyv>::cast(&feature))?;

            #[cfg(feature = "bytecheck")]
            let archived: &ArchivedString = rkyv::access(&bytes)?;

            #[cfg(not(feature = "bytecheck"))]
            let archived: &ArchivedString = unsafe { rkyv::access_unchecked(&bytes) };

            let deserialized: GuildFeature =
                rkyv::deserialize(With::<_, GuildFeatureRkyv>::cast(archived))?;

            assert_eq!(feature, deserialized);
        }

        Ok(())
    }
}
