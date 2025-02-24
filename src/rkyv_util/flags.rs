use std::marker::PhantomData;

use bitflags::Flags;
use rkyv::{
    niche::niching::Niching,
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

/// Used to archive bitflag types such as [`Permissions`] or [`MemberFlags`].
///
/// In case of bitflags wrapped in an [`Option`], instead of using
/// [`Map<BitflagsRkyv>`] you should be using
/// [`MapNiche<BitflagsRkyv, InvalidBitflags>`]; see
/// [`InvalidBitflags`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::flags::{BitflagsRkyv, InvalidBitflags};
/// use rkyv::with::{Map, MapNiche};
/// use twilight_model::guild::{MemberFlags, Permissions};
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = BitflagsRkyv)]
///     permissions: Permissions,
///     #[rkyv(with = MapNiche<BitflagsRkyv, InvalidBitflags>)]
///     member_flags: Option<MemberFlags>,
/// }
/// ```
///
/// [`Map<BitflagsRkyv>`]: rkyv::with::Map
/// [`MapNiche<BitflagsRkyv, InvalidBitflags>`]: rkyv::with::MapNiche
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
    ( $ty:ident ) => {
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

        impl<'a> Niching<ArchivedBitflags<$ty>> for InvalidBitflags
        where
            &'a [(); (Self::NICHED == $ty::all().bits()) as usize]: ValidBitflagNiching,
        {
            unsafe fn is_niched(niched: *const ArchivedBitflags<$ty>) -> bool {
                unsafe { (*niched).bits.to_native() == Self::NICHED }
            }

            fn resolve_niched(out: Place<ArchivedBitflags<$ty>>) {
                rkyv::munge::munge!(let ArchivedBitflags { bits, ..  } = out);
                Self::NICHED.resolve((), bits);
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

/// Used to niche bitflag types such as [`Permissions`] or [`MemberFlags`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::flags::{BitflagsRkyv, InvalidBitflags};
/// use rkyv::with::{Map, MapNiche};
/// use twilight_model::guild::{MemberFlags, Permissions};
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = MapNiche<BitflagsRkyv, InvalidBitflags>)]
///     permissions: Option<Permissions>,
///     #[rkyv(with = MapNiche<BitflagsRkyv, InvalidBitflags>)]
///     member_flags: Option<MemberFlags>,
/// }
///
/// // Same as above but without niching
/// #[derive(Archive)]
/// struct Naive {
///     #[rkyv(with = Map<BitflagsRkyv>)]
///     permissions: Option<Permissions>,
///     #[rkyv(with = Map<BitflagsRkyv>)]
///     member_flags: Option<MemberFlags>,
/// }
///
/// // Niching leverages bit-patterns to shrink the archived size
/// assert!(size_of::<ArchivedCached>() < size_of::<ArchivedNaive>());
/// ```
pub struct InvalidBitflags;

impl InvalidBitflags {
    const NICHED: Bits = u64::MAX;
}

/// Ensures that [`InvalidBitflags`] uses an adequate bit-pattern to niche a
/// bitflag type.
///
/// This code should fail to compile because the niching is invalid.
///
/// ```compile_fail
/// # trait ValidBitflagNiching {}
/// # impl<'a> ValidBitflagNiching for &'a [(); 0] {}
/// bitflags::bitflags! {
///     #[derive(Copy, Clone)]
///     pub struct MyFlags: u64 {
///         const A = u64::MAX;
///     }
/// }
///
/// fn is_valid<'a, T: ValidBitflagNiching>() {}
/// const _: fn() = is_valid::<&[(); (u64::MAX == MyFlags::all().bits()) as usize]>;
/// ```
///
/// This code should compile just fine.
///
/// ```no_run
/// # trait ValidBitflagNiching {}
/// # impl<'a> ValidBitflagNiching for &'a [(); 0] {}
/// bitflags::bitflags! {
///     #[derive(Copy, Clone)]
///     pub struct MyFlags: u64 {
///         const A = u64::MAX - 1;
///     }
/// }
///
/// fn is_valid<'a, T: ValidBitflagNiching>() {}
/// const _: fn() = is_valid::<&[(); (u64::MAX == MyFlags::all().bits()) as usize]>;
/// ```
trait ValidBitflagNiching {}

// Lifetime required due to <https://github.com/rust-lang/rust/issues/48214>
impl<'a> ValidBitflagNiching for &'a [(); 0] {}

#[cfg(test)]
mod tests {
    use rkyv::{
        niche::niched_option::NichedOption,
        rancor::Error,
        with::{MapNiche, With},
    };

    use super::*;

    #[test]
    fn test_rkyv_bitflags() -> Result<(), Error> {
        let flags = [
            Some(MemberFlags::COMPLETED_ONBOARDING | MemberFlags::DID_REJOIN),
            Some(MemberFlags::all()),
            Some(MemberFlags::empty()),
            None,
        ];

        for flags in flags {
            let bytes = rkyv::to_bytes(With::<_, MapNiche<BitflagsRkyv, InvalidBitflags>>::cast(
                &flags,
            ))?;

            #[cfg(feature = "bytecheck")]
            let archived: &NichedOption<
                ArchivedBitflags<MemberFlags>,
                InvalidBitflags,
            > = rkyv::access(&bytes)?;

            #[cfg(not(feature = "bytecheck"))]
            let archived: &NichedOption<
                ArchivedBitflags<MemberFlags>,
                InvalidBitflags,
            > = unsafe { rkyv::access_unchecked(&bytes) };

            let deserialized: Option<MemberFlags> =
                rkyv::deserialize(With::<_, MapNiche<BitflagsRkyv, InvalidBitflags>>::cast(
                    archived,
                ))?;

            assert_eq!(flags, deserialized);
        }

        Ok(())
    }
}
