use rkyv::{
    rancor::{Fallible, Source},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Place,
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
///     #[rkyv(with = TimestampRkyv)]
///     timestamp: Timestamp,
/// }
/// ```
pub struct TimestampRkyv;

impl TimestampRkyv {
    /// Turn a [`Timestamp`] into its archived form i.e. an `i64`.
    pub const fn archive(timestamp: &Timestamp) -> i64 {
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

    fn resolve_with(field: &Timestamp, resolver: Self::Resolver, out: Place<Self::Archived>) {
        Self::archive(field).resolve(resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Timestamp, S> for TimestampRkyv {
    fn serialize_with(_: &Timestamp, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D> DeserializeWith<Archived<i64>, Timestamp, D> for TimestampRkyv
where
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(archived: &Archived<i64>, _: &mut D) -> Result<Timestamp, D::Error> {
        Self::try_deserialize((*archived).into()).map_err(Source::new)
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_timestamp() -> Result<(), Error> {
        let timestamp = Timestamp::parse("2021-01-01T01:01:01.010000+00:00").unwrap();
        let bytes = rkyv::to_bytes(With::<_, TimestampRkyv>::cast(&timestamp))?;

        #[cfg(feature = "bytecheck")]
        let archived: &Archived<i64> = rkyv::access(&bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &Archived<i64> = unsafe { rkyv::access_unchecked(&bytes) };

        let deserialized: Timestamp = rkyv::deserialize(With::<_, TimestampRkyv>::cast(archived))?;

        assert_eq!(timestamp, deserialized);

        Ok(())
    }
}
