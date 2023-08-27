use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Fallible,
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
/// use rkyv::with::Map;
/// use twilight_model::guild::{MemberFlags, Permissions};
/// use twilight_redis::rkyv_util::util::BitflagsRkyv;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[with(BitflagsRkyv)]
///     permissions: Permissions,
///     #[with(Map<BitflagsRkyv>)]
///     member_flags: Option<MemberFlags>,
/// }
/// ```
pub struct BitflagsRkyv;

macro_rules! impl_bitflags {
    ( $ty:ident as $bits:ty ) => {
        impl ArchiveWith<$ty> for BitflagsRkyv {
            type Archived = Archived<$bits>;
            type Resolver = ();

            unsafe fn resolve_with(
                flags: &$ty,
                pos: usize,
                resolver: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                flags.bits().resolve(pos, resolver, out);
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

        impl<D: Fallible + ?Sized> DeserializeWith<Archived<$bits>, $ty, D> for BitflagsRkyv {
            fn deserialize_with(
                archived: &Archived<$bits>,
                _: &mut D,
            ) -> Result<$ty, <D as Fallible>::Error> {
                Ok($ty::from_bits_truncate((*archived).into()))
            }
        }
    };
}

impl_bitflags!(ActivityFlags as u64);
impl_bitflags!(ChannelFlags as u64);
impl_bitflags!(MemberFlags as u64);
impl_bitflags!(MessageFlags as u64);
impl_bitflags!(Permissions as u64);
impl_bitflags!(SystemChannelFlags as u64);
impl_bitflags!(UserFlags as u64);

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_bitflags() {
        type Wrapper = With<MemberFlags, BitflagsRkyv>;

        let flags = MemberFlags::COMPLETED_ONBOARDING | MemberFlags::DID_REJOIN;
        let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&flags)).unwrap();
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };
        let deserialized: MemberFlags =
            BitflagsRkyv::deserialize_with(archived, &mut Infallible).unwrap();

        assert_eq!(flags, deserialized);
    }
}
