use rkyv::{rancor::Fallible, Archive, Deserialize, Serialize};
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
#[derive(Archive, Serialize, Deserialize)]
#[rkyv(
    remote = Status,
    archived = ArchivedStatus,
    resolver = StatusResolver,
    derive(Copy, Clone, Debug, PartialEq, Eq),
)]
#[repr(u8)]
pub enum StatusRkyv {
    DoNotDisturb,
    Idle,
    Invisible,
    Offline,
    Online,
}

macro_rules! impl_from {
    ($ty:ident) => {
        impl From<$ty> for Status {
            fn from(status: $ty) -> Self {
                match status {
                    $ty::DoNotDisturb => Status::DoNotDisturb,
                    $ty::Idle => Status::Idle,
                    $ty::Invisible => Status::Invisible,
                    $ty::Offline => Status::Offline,
                    $ty::Online => Status::Online,
                }
            }
        }
    };
}

impl_from!(StatusRkyv);
impl_from!(ArchivedStatus);

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
            let archived: &ArchivedStatus = unsafe { rkyv::access_unchecked(&bytes) };

            #[cfg(feature = "bytecheck")]
            let archived: &ArchivedStatus = rkyv::access(&bytes)?;

            let deserialized: Status = rkyv::deserialize(With::<_, StatusRkyv>::cast(archived))?;

            assert_eq!(status, deserialized);
        }

        Ok(())
    }
}
