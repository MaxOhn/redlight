#![cfg(feature = "cold_resume")]

use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::BuildHasher,
};

use rkyv::{
    collections::util::Entry,
    out_field,
    ser::{ScratchSpace, Serializer},
    vec::{ArchivedVec, VecResolver},
    with::{ArchiveWith, DeserializeWith, RefAsBox, SerializeWith, With},
    Archive, Archived, Deserialize, Fallible, Resolver, Serialize,
};
use twilight_gateway::Session;

struct SessionRkyv;
pub(crate) struct SessionsRkyv;

#[derive(Archive, Serialize)]
pub(crate) struct SessionsWrapper<'a, S = RandomState> {
    #[with(SessionsRkyv)]
    sessions: &'a HashMap<u64, Session, S>,
}

pub(crate) struct ArchivedSession {
    id: Archived<Box<str>>,
    sequence: Archived<u64>,
}

struct SessionResolver {
    id: Resolver<Box<str>>,
}

impl ArchiveWith<Session> for SessionRkyv {
    type Archived = ArchivedSession;
    type Resolver = SessionResolver;

    unsafe fn resolve_with(
        session: &Session,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let id = session.id();
        let id = With::<_, RefAsBox>::cast(&id);
        let (fp, fo) = out_field!(out.id);
        id.resolve(pos + fp, resolver.id, fo);

        let sequence = session.sequence();
        let (fp, fo) = out_field!(out.sequence);
        sequence.resolve(pos + fp, (), fo);
    }
}

impl<S> SerializeWith<Session, S> for SessionRkyv
where
    S: Fallible + Serializer + ?Sized,
{
    fn serialize_with(
        session: &Session,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(SessionResolver {
            id: Serialize::serialize(With::<_, RefAsBox>::cast(&session.id()), serializer)?,
        })
    }
}

impl<D> DeserializeWith<ArchivedSession, Session, D> for SessionRkyv
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(archived: &ArchivedSession, d: &mut D) -> Result<Session, D::Error> {
        let id: Box<str> = archived.id.deserialize(d)?;
        let sequence = archived.sequence;

        // the .into() is necessary in case the `archive_le` or `archive_be`
        // features are enabled in rkyv
        #[allow(clippy::useless_conversion)]
        Ok(Session::new(sequence.into(), String::from(id)))
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use std::ptr::addr_of;

    use rkyv::{
        bytecheck::{ErrorBox, StructCheckError},
        CheckBytes,
    };

    impl<C: ?Sized> CheckBytes<C> for ArchivedSession
    where
        Archived<Box<str>>: CheckBytes<C>,
        Archived<u64>: CheckBytes<C>,
    {
        type Error = StructCheckError;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, StructCheckError> {
            <Archived<Box<str>> as CheckBytes<C>>::check_bytes(addr_of!((*value).id), context)
                .map_err(|e| StructCheckError {
                    field_name: "id",
                    inner: ErrorBox::new(e),
                })?;

            <Archived<u64> as CheckBytes<C>>::check_bytes(addr_of!((*value).sequence), context)
                .map_err(|e| StructCheckError {
                    field_name: "sequence",
                    inner: ErrorBox::new(e),
                })?;

            Ok(&*value)
        }
    }
};

impl<S> ArchiveWith<&HashMap<u64, Session, S>> for SessionsRkyv {
    type Archived = ArchivedVec<Entry<Archived<u64>, ArchivedSession>>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        map: &&HashMap<u64, Session, S>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(map.len(), pos, resolver, out);
    }
}

impl<H, S> SerializeWith<&HashMap<u64, Session, H>, S> for SessionsRkyv
where
    S: Fallible + ?Sized + Serializer + ScratchSpace,
{
    fn serialize_with(
        map: &&HashMap<u64, Session, H>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let iter = map.iter().map(|(shard_id, session)| Entry {
            // the .into() is necessary in case the `archive_le` or `archive_be`
            // features are enabled in rkyv
            #[allow(clippy::useless_conversion)]
            key: shard_id.into(),
            value: With::<_, SessionRkyv>::cast(session),
        });

        ArchivedVec::serialize_from_iter(iter, serializer)
    }
}

impl<S, D>
    DeserializeWith<ArchivedVec<Entry<Archived<u64>, ArchivedSession>>, HashMap<u64, Session, S>, D>
    for SessionsRkyv
where
    D: Fallible + ?Sized,
    S: BuildHasher + Default,
{
    fn deserialize_with(
        map: &ArchivedVec<Entry<Archived<u64>, ArchivedSession>>,
        d: &mut D,
    ) -> Result<HashMap<u64, Session, S>, D::Error> {
        map.iter()
            .map(|Entry { key, value }| {
                SessionRkyv::deserialize_with(value, d).map(|session| {
                    // the .into() is necessary in case the `archive_le` or `archive_be`
                    // features are enabled in rkyv
                    #[allow(clippy::useless_conversion)]
                    ((*key).into(), session)
                })
            })
            .collect()
    }
}

impl<'a, S> SessionsWrapper<'a, S> {
    pub(crate) const fn new(sessions: &'a HashMap<u64, Session, S>) -> Self {
        Self { sessions }
    }
}

impl<'a, D, S> Deserialize<HashMap<u64, Session, S>, D> for ArchivedSessionsWrapper<'a, S>
where
    D: Fallible + ?Sized,
    S: BuildHasher + Default,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<HashMap<u64, Session, S>, D::Error> {
        SessionsRkyv::deserialize_with(&self.sessions, deserializer)
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use std::{error::Error as StdError, ptr::addr_of};

    use rkyv::{
        validation::{owned::CheckOwnedPointerError, ArchiveContext},
        CheckBytes,
    };

    impl<'a, S, C> CheckBytes<C> for ArchivedSessionsWrapper<'a, S>
    where
        C: ArchiveContext + ?Sized,
        C::Error: StdError + Send + Sync,
    {
        type Error = CheckOwnedPointerError<[Entry<u64, ArchivedSession>], C>;

        unsafe fn check_bytes<'bytecheck>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'bytecheck Self, Self::Error> {
            ArchivedVec::<Entry<Archived<u64>, ArchivedSession>>::check_bytes(
                addr_of!((*value).sessions),
                context,
            )?;

            Ok(&*value)
        }
    }
};

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    fn session() -> Session {
        Session::new(123, "session_id".to_owned())
    }

    #[test]
    fn test_rkyv_session() {
        type Wrapper = With<Session, SessionRkyv>;

        let session = session();
        let bytes = rkyv::to_bytes::<_, 16>(Wrapper::cast(&session)).unwrap();

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

        let deserialized: Session =
            SessionRkyv::deserialize_with(archived, &mut Infallible).unwrap();

        assert_eq!(session, deserialized);
    }

    #[test]
    fn test_rkyv_sessions() {
        let sessions: HashMap<_, _> = (0..).zip(std::iter::repeat(session()).take(10)).collect();
        let wrapper = SessionsWrapper::new(&sessions);
        let bytes = rkyv::to_bytes::<_, 64>(&wrapper).unwrap();

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<SessionsWrapper>(&bytes) };

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<SessionsWrapper>(&bytes).unwrap();

        let deserialized: HashMap<u64, Session> = archived.deserialize(&mut Infallible).unwrap();

        assert_eq!(sessions, deserialized);
    }
}
