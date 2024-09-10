use rkyv::{
    rancor::Fallible,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Place,
};
use twilight_model::{
    channel::{message::MessageFlags, ChannelFlags},
    gateway::presence::ActivityFlags,
    guild::{MemberFlags, Permissions, SystemChannelFlags},
    user::UserFlags,
};

/// Used to archive flag type such as [`Permissions`] or [`MemberFlags`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::util::BitflagsRkyv;
/// use rkyv::with::Map;
/// use twilight_model::guild::{MemberFlags, Permissions};
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = BitflagsRkyv)]
///     permissions: Permissions,
///     #[rkyv(with = Map<BitflagsRkyv>)]
///     member_flags: Option<MemberFlags>,
/// }
/// ```
pub struct BitflagsRkyv;

macro_rules! impl_bitflags {
    ($ty:ident) => {
        impl ArchiveWith<$ty> for BitflagsRkyv {
            type Archived = Archived<u64>;
            type Resolver = ();

            fn resolve_with(flags: &$ty, resolver: Self::Resolver, out: Place<Self::Archived>) {
                flags.bits().resolve(resolver, out);
            }
        }

        impl<S: Fallible + ?Sized> SerializeWith<$ty, S> for BitflagsRkyv {
            fn serialize_with(
                _: &$ty,
                _: &mut S,
            ) -> Result<Self::Resolver, <S as Fallible>::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> DeserializeWith<Archived<u64>, $ty, D> for BitflagsRkyv {
            fn deserialize_with(
                archived: &Archived<u64>,
                _: &mut D,
            ) -> Result<$ty, <D as Fallible>::Error> {
                Ok($ty::from_bits_truncate((*archived).into()))
            }
        }
    };
}

impl_bitflags!(ActivityFlags);
impl_bitflags!(ChannelFlags);
impl_bitflags!(MemberFlags);
impl_bitflags!(MessageFlags);
impl_bitflags!(Permissions);
impl_bitflags!(SystemChannelFlags);
impl_bitflags!(UserFlags);

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_bitflags() -> Result<(), Error> {
        let flags = MemberFlags::COMPLETED_ONBOARDING | MemberFlags::DID_REJOIN;
        let bytes = rkyv::to_bytes(With::<_, BitflagsRkyv>::cast(&flags))?;

        #[cfg(feature = "bytecheck")]
        let archived: &Archived<u64> = rkyv::access(&bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &Archived<u64> = unsafe { rkyv::access_unchecked(&bytes) };

        let deserialized: MemberFlags = rkyv::deserialize(With::<_, BitflagsRkyv>::cast(archived))?;

        assert_eq!(flags, deserialized);

        Ok(())
    }
}
