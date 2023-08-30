use rkyv::{
    ser::Serializer,
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Fallible,
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
///     #[with(GuildFeatureRkyv)]
///     as_owned: GuildFeature,
///     #[with(GuildFeatureRkyv)]
///     as_ref: &'a GuildFeature,
/// }
/// ```
pub struct GuildFeatureRkyv;

impl ArchiveWith<GuildFeature> for GuildFeatureRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    unsafe fn resolve_with(
        feature: &GuildFeature,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedString::resolve_from_str(guild_feature_str(feature), pos, resolver, out)
    }
}

impl<S: Fallible + Serializer + ?Sized> SerializeWith<GuildFeature, S> for GuildFeatureRkyv {
    fn serialize_with(
        feature: &GuildFeature,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedString::serialize_from_str(guild_feature_str(feature), serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedString, GuildFeature, D> for GuildFeatureRkyv {
    fn deserialize_with(
        archived: &ArchivedString,
        _: &mut D,
    ) -> Result<GuildFeature, <D as Fallible>::Error> {
        #[allow(deprecated)]
        let feature = match archived.as_str() {
            GUILD_FEATURE_ANIMATED_BANNER => GuildFeature::AnimatedBanner,
            GUILD_FEATURE_ANIMATED_ICON => GuildFeature::AnimatedIcon,
            GUILD_FEATURE_AUTO_MODERATION => GuildFeature::AutoModeration,
            GUILD_FEATURE_BANNER => GuildFeature::Banner,
            GUILD_FEATURE_COMMERCE => GuildFeature::Commerce,
            GUILD_FEATURE_COMMUNITY => GuildFeature::Community,
            GUILD_FEATURE_CREATOR_MONETIZABLE_PROVISIONAL => {
                GuildFeature::CreatorMonetizableProvisional
            }
            GUILD_FEATURE_CREATOR_STORE_PAGE => GuildFeature::CreatorStorePage,
            GUILD_FEATURE_DEVELOPER_SUPPORT_SERVER => GuildFeature::DeveloperSupportServer,
            GUILD_FEATURE_DISCOVERABLE => GuildFeature::Discoverable,
            GUILD_FEATURE_FEATURABLE => GuildFeature::Featurable,
            GUILD_FEATURE_INVITES_DISABLED => GuildFeature::InvitesDisabled,
            GUILD_FEATURE_INVITE_SPLASH => GuildFeature::InviteSplash,
            GUILD_FEATURE_MEMBER_VERIFICATION_GATE_ENABLED => {
                GuildFeature::MemberVerificationGateEnabled
            }
            GUILD_FEATURE_MONETIZATION_ENABLED => GuildFeature::MonetizationEnabled,
            GUILD_FEATURE_MORE_STICKERS => GuildFeature::MoreStickers,
            GUILD_FEATURE_NEWS => GuildFeature::News,
            GUILD_FEATURE_PARTNERED => GuildFeature::Partnered,
            GUILD_FEATURE_PREVIEW_ENABLED => GuildFeature::PreviewEnabled,
            GUILD_FEATURE_PRIVATE_THREADS => GuildFeature::PrivateThreads,
            GUILD_FEATURE_ROLE_ICONS => GuildFeature::RoleIcons,
            GUILD_FEATURE_ROLE_SUBSCRIPTIONS_AVAILABLE_FOR_PURCHASE => {
                GuildFeature::RoleSubscriptionsAvailableForPurchase
            }
            GUILD_FEATURE_ROLE_SUBSCRIPTIONS_ENABLED => GuildFeature::RoleSubscriptionsEnabled,
            GUILD_FEATURE_TICKETED_EVENTS_ENABLED => GuildFeature::TicketedEventsEnabled,
            GUILD_FEATURE_VANITY_URL => GuildFeature::VanityUrl,
            GUILD_FEATURE_VERIFIED => GuildFeature::Verified,
            GUILD_FEATURE_VIP_REGIONS => GuildFeature::VipRegions,
            GUILD_FEATURE_WELCOME_SCREEN_ENABLED => GuildFeature::WelcomeScreenEnabled,
            unknown => GuildFeature::Unknown(unknown.to_owned()),
        };

        Ok(feature)
    }
}

impl ArchiveWith<&GuildFeature> for GuildFeatureRkyv {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    unsafe fn resolve_with(
        feature: &&GuildFeature,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        <Self as ArchiveWith<GuildFeature>>::resolve_with(feature, pos, resolver, out);
    }
}

impl<S: Fallible + Serializer + ?Sized> SerializeWith<&GuildFeature, S> for GuildFeatureRkyv {
    fn serialize_with(
        feature: &&GuildFeature,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Self as SerializeWith<GuildFeature, S>>::serialize_with(feature, serializer)
    }
}

const GUILD_FEATURE_ANIMATED_BANNER: &str = "ANIMATED_BANNER";
const GUILD_FEATURE_ANIMATED_ICON: &str = "ANIMATED_ICON";
const GUILD_FEATURE_AUTO_MODERATION: &str = "AUTO_MODERATION";
const GUILD_FEATURE_BANNER: &str = "BANNER";
const GUILD_FEATURE_COMMERCE: &str = "COMMERCE";
const GUILD_FEATURE_COMMUNITY: &str = "COMMUNITY";
const GUILD_FEATURE_CREATOR_MONETIZABLE_PROVISIONAL: &str = "CREATOR_MONETIZABLE_PROVISIONAL";
const GUILD_FEATURE_CREATOR_STORE_PAGE: &str = "CREATOR_STORE_PAGE";
const GUILD_FEATURE_DEVELOPER_SUPPORT_SERVER: &str = "DEVELOPER_SUPPORT_SERVER";
const GUILD_FEATURE_DISCOVERABLE: &str = "DISCOVERABLE";
const GUILD_FEATURE_FEATURABLE: &str = "FEATURABLE";
const GUILD_FEATURE_INVITES_DISABLED: &str = "INVITES_DISABLED";
const GUILD_FEATURE_INVITE_SPLASH: &str = "INVITE_SPLASH";
const GUILD_FEATURE_MEMBER_VERIFICATION_GATE_ENABLED: &str = "MEMBER_VERIFICATION_GATE_ENABLED";
const GUILD_FEATURE_MONETIZATION_ENABLED: &str = "MONETIZATION_ENABLED";
const GUILD_FEATURE_MORE_STICKERS: &str = "MORE_STICKERS";
const GUILD_FEATURE_NEWS: &str = "NEWS";
const GUILD_FEATURE_PARTNERED: &str = "PARTNERED";
const GUILD_FEATURE_PREVIEW_ENABLED: &str = "PREVIEW_ENABLED";
const GUILD_FEATURE_PRIVATE_THREADS: &str = "PRIVATE_THREADS";
const GUILD_FEATURE_ROLE_ICONS: &str = "ROLE_ICONS";
const GUILD_FEATURE_ROLE_SUBSCRIPTIONS_AVAILABLE_FOR_PURCHASE: &str =
    "ROLE_SUBSCRIPTIONS_AVAILABLE_FOR_PURCHASE";
const GUILD_FEATURE_ROLE_SUBSCRIPTIONS_ENABLED: &str = "ROLE_SUBSCRIPTIONS_ENABLED";
const GUILD_FEATURE_TICKETED_EVENTS_ENABLED: &str = "TICKETED_EVENTS_ENABLED";
const GUILD_FEATURE_VANITY_URL: &str = "VANITY_URL";
const GUILD_FEATURE_VERIFIED: &str = "VERIFIED";
const GUILD_FEATURE_VIP_REGIONS: &str = "VIP_REGIONS";
const GUILD_FEATURE_WELCOME_SCREEN_ENABLED: &str = "WELCOME_SCREEN_ENABLED";

fn guild_feature_str(feature: &GuildFeature) -> &str {
    #[allow(deprecated)]
    match feature {
        GuildFeature::AnimatedBanner => GUILD_FEATURE_ANIMATED_BANNER,
        GuildFeature::AnimatedIcon => GUILD_FEATURE_ANIMATED_ICON,
        GuildFeature::AutoModeration => GUILD_FEATURE_AUTO_MODERATION,
        GuildFeature::Banner => GUILD_FEATURE_BANNER,
        GuildFeature::Commerce => GUILD_FEATURE_COMMERCE,
        GuildFeature::Community => GUILD_FEATURE_COMMUNITY,
        GuildFeature::CreatorMonetizableProvisional => {
            GUILD_FEATURE_CREATOR_MONETIZABLE_PROVISIONAL
        }
        GuildFeature::CreatorStorePage => GUILD_FEATURE_CREATOR_STORE_PAGE,
        GuildFeature::DeveloperSupportServer => GUILD_FEATURE_DEVELOPER_SUPPORT_SERVER,
        GuildFeature::Discoverable => GUILD_FEATURE_DISCOVERABLE,
        GuildFeature::Featurable => GUILD_FEATURE_FEATURABLE,
        GuildFeature::InvitesDisabled => GUILD_FEATURE_INVITES_DISABLED,
        GuildFeature::InviteSplash => GUILD_FEATURE_INVITE_SPLASH,
        GuildFeature::MemberVerificationGateEnabled => {
            GUILD_FEATURE_MEMBER_VERIFICATION_GATE_ENABLED
        }
        GuildFeature::MonetizationEnabled => GUILD_FEATURE_MONETIZATION_ENABLED,
        GuildFeature::MoreStickers => GUILD_FEATURE_MORE_STICKERS,
        GuildFeature::News => GUILD_FEATURE_NEWS,
        GuildFeature::Partnered => GUILD_FEATURE_PARTNERED,
        GuildFeature::PreviewEnabled => GUILD_FEATURE_PREVIEW_ENABLED,
        GuildFeature::PrivateThreads => GUILD_FEATURE_PRIVATE_THREADS,
        GuildFeature::RoleIcons => GUILD_FEATURE_ROLE_ICONS,
        GuildFeature::RoleSubscriptionsAvailableForPurchase => {
            GUILD_FEATURE_ROLE_SUBSCRIPTIONS_AVAILABLE_FOR_PURCHASE
        }
        GuildFeature::RoleSubscriptionsEnabled => GUILD_FEATURE_ROLE_SUBSCRIPTIONS_ENABLED,
        GuildFeature::TicketedEventsEnabled => GUILD_FEATURE_TICKETED_EVENTS_ENABLED,
        GuildFeature::VanityUrl => GUILD_FEATURE_VANITY_URL,
        GuildFeature::Verified => GUILD_FEATURE_VERIFIED,
        GuildFeature::VipRegions => GUILD_FEATURE_VIP_REGIONS,
        GuildFeature::WelcomeScreenEnabled => GUILD_FEATURE_WELCOME_SCREEN_ENABLED,
        GuildFeature::Unknown(unknown) => unknown.as_str(),
        _ => "non_exhaustive",
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_guild_feature() {
        type Wrapper = With<GuildFeature, GuildFeatureRkyv>;

        let features = [
            GuildFeature::Banner,
            GuildFeature::Unknown("other".to_owned()),
        ];

        for feature in features {
            let bytes = rkyv::to_bytes::<_, 16>(Wrapper::cast(&feature)).unwrap();

            #[cfg(feature = "validation")]
            let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

            #[cfg(not(feature = "validation"))]
            let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

            let deserialized: GuildFeature =
                GuildFeatureRkyv::deserialize_with(archived, &mut Infallible).unwrap();

            assert_eq!(feature, deserialized);
        }
    }
}
