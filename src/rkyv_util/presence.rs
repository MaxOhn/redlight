use rkyv::{
    rancor::Fallible,
    traits::NoUndef,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Deserialize, Place, Portable,
};
use twilight_model::gateway::presence::Status;

/// Used to archive [`Status`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use redlight::rkyv_util::presence::StatusRkyv;
/// use twilight_model::gateway::presence::Status;
///
/// #[derive(Archive)]
/// struct Cached {
///     #[rkyv(with = StatusRkyv)]
///     status: Status,
/// }
/// ```
pub struct StatusRkyv;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(rkyv::bytecheck::CheckBytes),
    bytecheck(crate = rkyv::bytecheck),
)]
#[repr(u8)]
/// An archived [`Status`].
pub enum ArchivedStatus {
    DoNotDisturb,
    Idle,
    Invisible,
    Offline,
    Online,
}

impl From<ArchivedStatus> for Status {
    fn from(archived: ArchivedStatus) -> Self {
        match archived {
            ArchivedStatus::DoNotDisturb => Self::DoNotDisturb,
            ArchivedStatus::Idle => Self::Idle,
            ArchivedStatus::Invisible => Self::Invisible,
            ArchivedStatus::Offline => Self::Offline,
            ArchivedStatus::Online => Self::Online,
        }
    }
}

impl From<Status> for ArchivedStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::DoNotDisturb => Self::DoNotDisturb,
            Status::Idle => Self::Idle,
            Status::Invisible => Self::Invisible,
            Status::Offline => Self::Offline,
            Status::Online => Self::Online,
        }
    }
}

unsafe impl NoUndef for ArchivedStatus {}

impl ArchiveWith<Status> for StatusRkyv {
    type Archived = ArchivedStatus;
    type Resolver = ();

    fn resolve_with(status: &Status, (): Self::Resolver, out: Place<Self::Archived>) {
        out.write(ArchivedStatus::from(*status));
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Status, S> for StatusRkyv {
    fn serialize_with(_: &Status, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedStatus, Status, D> for StatusRkyv {
    fn deserialize_with(
        archived: &ArchivedStatus,
        deserializer: &mut D,
    ) -> Result<Status, D::Error> {
        archived.deserialize(deserializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<Status, D> for ArchivedStatus {
    fn deserialize(&self, _: &mut D) -> Result<Status, <D as Fallible>::Error> {
        Ok(Status::from(*self))
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_status() -> Result<(), Error> {
        let statuses = [
            Status::DoNotDisturb,
            Status::Idle,
            Status::Invisible,
            Status::Offline,
            Status::Online,
        ];

        for status in statuses {
            let bytes = rkyv::to_bytes(With::<_, StatusRkyv>::cast(&status))?;

            #[cfg(not(feature = "bytecheck"))]
            let archived = unsafe { rkyv::access_unchecked(&bytes) };

            #[cfg(feature = "bytecheck")]
            let archived = rkyv::access(&bytes)?;

            let deserialized: Status = rkyv::deserialize(With::<_, StatusRkyv>::cast(archived))?;

            assert_eq!(status, deserialized);
        }

        Ok(())
    }
}
