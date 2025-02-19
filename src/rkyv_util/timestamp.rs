use rkyv::{
    rancor::{Fallible, Source},
    traits::NoUndef,
    with::DeserializeWith,
    Archive, Archived, Deserialize, Serialize,
};
use twilight_model::util::{datetime::TimestampParseError, Timestamp};

/// Used to archive [`Timestamp`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::timestamp::TimestampRkyv;
/// use twilight_model::util::Timestamp;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = TimestampRkyv)]
///     timestamp: Timestamp,
/// }
/// ```
#[derive(Archive, Serialize)]
#[rkyv(
    remote = Timestamp,
    archived = ArchivedTimestamp,
    resolver = TimestampResolver,
    derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord),
)]
#[repr(transparent)]
pub struct TimestampRkyv(#[rkyv(getter = as_micros)] Micros);

type Micros = i64;

const fn as_micros(timestamp: &Timestamp) -> Micros {
    timestamp.as_micros()
}

// SAFETY: `ArchivedTimestamp` is a wrapper of `Archived<Micros>`.
unsafe impl NoUndef for ArchivedTimestamp where Archived<Micros>: NoUndef {}

impl ArchivedTimestamp {
    /// Create a new [`ArchivedTimestamp`].
    pub const fn new(timestamp: &Timestamp) -> Self {
        Self(<Archived<Micros>>::from_native(as_micros(timestamp)))
    }

    /// Try to convert a [`ArchivedTimestamp`] into a [`Timestamp`].
    pub fn try_deserialize(self) -> Result<Timestamp, TimestampParseError> {
        Timestamp::from_micros(self.0.to_native())
    }
}

impl<D> DeserializeWith<ArchivedTimestamp, Timestamp, D> for TimestampRkyv
where
    D: Fallible<Error: Source> + ?Sized,
{
    fn deserialize_with(
        timestamp: &ArchivedTimestamp,
        d: &mut D,
    ) -> Result<Timestamp, <D as Fallible>::Error> {
        timestamp.deserialize(d)
    }
}

impl<D> Deserialize<Timestamp, D> for ArchivedTimestamp
where
    D: Fallible<Error: Source> + ?Sized,
{
    fn deserialize(&self, _: &mut D) -> Result<Timestamp, <D as Fallible>::Error> {
        self.try_deserialize().map_err(Source::new)
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
        let archived: &ArchivedTimestamp = rkyv::access(&bytes)?;

        #[cfg(not(feature = "bytecheck"))]
        let archived: &ArchivedTimestamp = unsafe { rkyv::access_unchecked(&bytes) };

        let deserialized: Timestamp = rkyv::deserialize(With::<_, TimestampRkyv>::cast(archived))?;

        assert_eq!(timestamp, deserialized);

        Ok(())
    }
}
