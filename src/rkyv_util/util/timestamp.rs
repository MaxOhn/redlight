use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Fallible,
};
use twilight_model::util::{datetime::TimestampParseError, Timestamp};

/// Used to archive [`Timestamp`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::util::TimestampRkyv;
/// use twilight_model::util::Timestamp;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[with(TimestampRkyv)]
///     timestamp: Timestamp,
/// }
/// ```
pub struct TimestampRkyv;

impl TimestampRkyv {
    /// Turn a [`Timestamp`] into its archived form i.e. an `i64`.
    pub fn archive(timestamp: &Timestamp) -> i64 {
        timestamp.as_micros()
    }

    /// Consider an `i64` as a [`Timestamp`] archive and try to convert it.
    pub fn try_deserialize(timestamp: i64) -> Result<Timestamp, TimestampParseError> {
        Timestamp::from_micros(timestamp)
    }
}

impl ArchiveWith<Timestamp> for TimestampRkyv {
    type Archived = Archived<i64>;
    type Resolver = ();

    unsafe fn resolve_with(
        field: &Timestamp,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        Self::archive(field).resolve(pos, resolver, out)
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Timestamp, S> for TimestampRkyv {
    fn serialize_with(_: &Timestamp, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D> DeserializeWith<Archived<i64>, Timestamp, D> for TimestampRkyv
where
    D: Fallible + ?Sized,
    D::Error: From<TimestampParseError>,
{
    fn deserialize_with(
        archived: &Archived<i64>,
        _: &mut D,
    ) -> Result<Timestamp, <D as Fallible>::Error> {
        // the .into() is necessary in case the `archive_le` or `archive_be`
        // features are enabled in rkyv
        #[allow(clippy::useless_conversion)]
        Ok(Self::try_deserialize((*archived).into())?)
    }
}

#[cfg(test)]
mod tests {
    use rkyv::with::With;

    use super::*;

    #[test]
    fn test_rkyv_timestamp() {
        struct TimestampDeserializer;

        #[derive(Debug)]
        struct TimestampDeserializerError;

        impl Fallible for TimestampDeserializer {
            type Error = TimestampDeserializerError;
        }

        impl From<TimestampParseError> for TimestampDeserializerError {
            fn from(_: TimestampParseError) -> Self {
                Self
            }
        }

        type Wrapper = With<Timestamp, TimestampRkyv>;

        let timestamp = Timestamp::parse("2021-01-01T01:01:01.010000+00:00").unwrap();
        let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&timestamp)).unwrap();
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };
        let deserialized: Timestamp =
            TimestampRkyv::deserialize_with(archived, &mut TimestampDeserializer).unwrap();

        assert_eq!(timestamp, deserialized);
    }
}
