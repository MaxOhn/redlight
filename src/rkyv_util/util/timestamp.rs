use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Fallible,
};
use twilight_model::util::{datetime::TimestampParseError, Timestamp};

/// Used to archive [`Timestamp`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use twilight_model::util::Timestamp;
/// use twilight_redis::rkyv_util::util::TimestampRkyv;
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
    type Archived = i64;
    type Resolver = ();

    unsafe fn resolve_with(
        field: &Timestamp,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        Archive::resolve(&Self::archive(field), pos, resolver, out)
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Timestamp, S> for TimestampRkyv {
    fn serialize_with(_: &Timestamp, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D> DeserializeWith<i64, Timestamp, D> for TimestampRkyv
where
    D: Fallible + ?Sized,
    D::Error: From<TimestampParseError>,
{
    fn deserialize_with(field: &i64, _: &mut D) -> Result<Timestamp, <D as Fallible>::Error> {
        Ok(Self::try_deserialize(*field)?)
    }
}
