use std::marker::PhantomData;

use bitflags::Flags;
use rkyv::{
    rancor::Fallible,
    traits::NoUndef,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Place, Portable,
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
/// use redlight::rkyv_util::flags::BitflagsRkyv;
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

/// Archived bitflags.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(rkyv::bytecheck::CheckBytes),
    bytecheck(crate = rkyv::bytecheck),
)]
#[repr(transparent)]
pub struct ArchivedBitflags<T> {
    pub bits: Archived<Bits>,
    _phantom: PhantomData<T>,
}

type Bits = u64;

// SAFETY: `ArchivedBitflags<_>` is a wrapper of `Archived<Bits>`.
unsafe impl<T> NoUndef for ArchivedBitflags<T> where Archived<Bits>: NoUndef {}

impl<T> ArchivedBitflags<T> {
    /// Create new [`ArchivedBitflags`].
    pub const fn new(bits: Bits) -> Self {
        Self {
            bits: <Archived<Bits>>::from_native(bits),
            _phantom: PhantomData,
        }
    }
}

impl<T: Flags<Bits = Bits>> ArchivedBitflags<T> {
    pub fn to_native(self) -> T {
        T::from_bits_truncate(self.bits.to_native())
    }
}

impl<T: Flags<Bits = Bits>> From<T> for ArchivedBitflags<T> {
    fn from(flags: T) -> Self {
        Self::new(flags.bits())
    }
}

impl<T: Flags<Bits = Bits>> PartialEq<T> for ArchivedBitflags<T> {
    fn eq(&self, other: &T) -> bool {
        self.bits.to_native() == other.bits()
    }
}

macro_rules! impl_bitflags {
    ($ty:ident) => {
        impl ArchiveWith<$ty> for BitflagsRkyv {
            type Archived = ArchivedBitflags<$ty>;
            type Resolver = ();

            fn resolve_with(flags: &$ty, resolver: Self::Resolver, out: Place<Self::Archived>) {
                rkyv::munge::munge!(let ArchivedBitflags { bits, .. } = out);
                flags.bits().resolve(resolver, bits);
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

        impl<D: Fallible + ?Sized> DeserializeWith<ArchivedBitflags<$ty>, $ty, D> for BitflagsRkyv {
            fn deserialize_with(
                archived: &ArchivedBitflags<$ty>,
                d: &mut D,
            ) -> Result<$ty, <D as Fallible>::Error> {
                archived.deserialize(d)
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<$ty, D> for ArchivedBitflags<$ty> {
            fn deserialize(&self, _: &mut D) -> Result<$ty, <D as Fallible>::Error> {
                Ok(self.to_native())
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
        let archived: &ArchivedBitflags<MemberFlags> = rkyv::access(&bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &ArchivedBitflags<MemberFlags> = unsafe { rkyv::access_unchecked(&bytes) };

        let deserialized: MemberFlags = rkyv::deserialize(With::<_, BitflagsRkyv>::cast(archived))?;

        assert_eq!(flags, deserialized);

        Ok(())
    }
}
