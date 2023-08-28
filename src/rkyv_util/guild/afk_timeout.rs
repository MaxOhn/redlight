use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Fallible,
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
///     #[with(AfkTimeoutRkyv)]
///     afk_timeout: AfkTimeout,
/// }
/// ```
pub struct AfkTimeoutRkyv;

impl ArchiveWith<AfkTimeout> for AfkTimeoutRkyv {
    type Archived = Archived<u16>;
    type Resolver = ();

    unsafe fn resolve_with(
        timeout: &AfkTimeout,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        timeout.get().resolve(pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<AfkTimeout, S> for AfkTimeoutRkyv {
    fn serialize_with(_: &AfkTimeout, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<u16>, AfkTimeout, D> for AfkTimeoutRkyv {
    fn deserialize_with(
        archived: &Archived<u16>,
        _: &mut D,
    ) -> Result<AfkTimeout, <D as Fallible>::Error> {
        // the `from` is necessary in case the `archive_le` or `archive_be`
        // features are enabled in rkyv
        #[allow(clippy::useless_conversion)]
        Ok(u16::from(*archived).into())
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_afk_timeout() {
        type Wrapper = With<AfkTimeout, AfkTimeoutRkyv>;

        let timeouts = [AfkTimeout::FIFTEEN_MINUTES, AfkTimeout::from(12345_u16)];

        for timeout in timeouts {
            let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&timeout)).unwrap();
            let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };
            let deserialized: AfkTimeout =
                AfkTimeoutRkyv::deserialize_with(archived, &mut Infallible).unwrap();

            assert_eq!(timeout, deserialized);
        }
    }
}
