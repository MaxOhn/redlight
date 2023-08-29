use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Deserialize, Fallible,
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
///     #[with(StatusRkyv)]
///     status: Status,
/// }
/// ```
pub struct StatusRkyv;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

impl ArchiveWith<Status> for StatusRkyv {
    type Archived = ArchivedStatus;
    type Resolver = ();

    unsafe fn resolve_with(status: &Status, _: usize, _: Self::Resolver, out: *mut Self::Archived) {
        out.write(ArchivedStatus::from(*status));
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Status, S> for StatusRkyv {
    fn serialize_with(_: &Status, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedStatus, Status, D> for StatusRkyv {
    fn deserialize_with(
        archived: &ArchivedStatus,
        deserializer: &mut D,
    ) -> Result<Status, <D as Fallible>::Error> {
        archived.deserialize(deserializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<Status, D> for ArchivedStatus {
    fn deserialize(&self, _: &mut D) -> Result<Status, <D as Fallible>::Error> {
        Ok(Status::from(*self))
    }
}

#[cfg(feature = "validation")]
#[cfg_attr(docsrs, doc(cfg(feature = "validation")))]
const _: () = {
    use rkyv::{bytecheck::EnumCheckError, CheckBytes};

    struct Discriminant;

    #[allow(non_upper_case_globals)]
    impl Discriminant {
        const DoNotDisturb: u8 = ArchivedStatus::DoNotDisturb as u8;
        const Idle: u8 = ArchivedStatus::Idle as u8;
        const Invisible: u8 = ArchivedStatus::Invisible as u8;
        const Offline: u8 = ArchivedStatus::Offline as u8;
        const Online: u8 = ArchivedStatus::Online as u8;
    }

    impl<C: ?Sized> CheckBytes<C> for ArchivedStatus {
        type Error = EnumCheckError<u8>;

        unsafe fn check_bytes<'bytecheck>(
            value: *const Self,
            _: &mut C,
        ) -> Result<&'bytecheck Self, EnumCheckError<u8>> {
            let tag = *value.cast::<u8>();

            match tag {
                Discriminant::DoNotDisturb => {}
                Discriminant::Idle => {}
                Discriminant::Invisible => {}
                Discriminant::Offline => {}
                Discriminant::Online => {}
                _ => return Err(EnumCheckError::InvalidTag(tag)),
            }

            Ok(&*value)
        }
    }
};

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_status() {
        type Wrapper = With<Status, StatusRkyv>;

        let statuses = [
            Status::DoNotDisturb,
            Status::Idle,
            Status::Invisible,
            Status::Offline,
            Status::Online,
        ];

        for status in statuses {
            let bytes = rkyv::to_bytes::<_, 0>(Wrapper::cast(&status)).unwrap();

            #[cfg(not(feature = "validation"))]
            let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

            #[cfg(feature = "validation")]
            let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

            let deserialized: Status = archived.deserialize(&mut Infallible).unwrap();

            assert_eq!(status, deserialized);
        }
    }
}
