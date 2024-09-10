use rkyv::{
    rancor::Fallible,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Place,
};
use twilight_model::guild::AfkTimeout;

/// Used to archive [`AfkTimeout`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::guild::AfkTimeoutRkyv;
/// use twilight_model::guild::AfkTimeout;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = AfkTimeoutRkyv)]
///     afk_timeout: AfkTimeout,
/// }
/// ```
pub struct AfkTimeoutRkyv;

impl ArchiveWith<AfkTimeout> for AfkTimeoutRkyv {
    type Archived = Archived<u16>;
    type Resolver = ();

    fn resolve_with(timeout: &AfkTimeout, resolver: Self::Resolver, out: Place<Self::Archived>) {
        timeout.get().resolve(resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<AfkTimeout, S> for AfkTimeoutRkyv {
    fn serialize_with(_: &AfkTimeout, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<u16>, AfkTimeout, D> for AfkTimeoutRkyv {
    fn deserialize_with(archived: &Archived<u16>, _: &mut D) -> Result<AfkTimeout, D::Error> {
        Ok(u16::from(*archived).into())
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_afk_timeout() -> Result<(), Error> {
        let timeouts = [AfkTimeout::FIFTEEN_MINUTES, AfkTimeout::from(12345_u16)];

        for timeout in timeouts {
            let bytes = rkyv::to_bytes(With::<_, AfkTimeoutRkyv>::cast(&timeout))?;

            #[cfg(feature = "bytecheck")]
            let archived: &Archived<u16> = rkyv::access(&bytes)?;

            #[cfg(not(feature = "bytecheck"))]
            let archived: &Archived<u16> = unsafe { rkyv::access_unchecked(&bytes) };

            let deserialized: AfkTimeout =
                rkyv::deserialize(With::<_, AfkTimeoutRkyv>::cast(archived))?;

            assert_eq!(timeout, deserialized);
        }

        Ok(())
    }
}
