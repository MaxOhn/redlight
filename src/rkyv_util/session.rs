#![cfg(feature = "cold_resume")]

use std::{collections::HashMap, hash::BuildHasher};

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
pub(crate) struct SessionsWrapper<'a, S> {
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

        Ok(Session::new(sequence, String::from(id)))
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
    type Archived = ArchivedVec<Entry<u64, ArchivedSession>>;
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
            key: shard_id,
            value: With::<_, SessionRkyv>::cast(session),
        });

        ArchivedVec::serialize_from_iter(iter, serializer)
    }
}

impl<S, D> DeserializeWith<ArchivedVec<Entry<u64, ArchivedSession>>, HashMap<u64, Session, S>, D>
    for SessionsRkyv
where
    D: Fallible + ?Sized,
    S: BuildHasher + Default,
{
    fn deserialize_with(
        map: &ArchivedVec<Entry<u64, ArchivedSession>>,
        d: &mut D,
    ) -> Result<HashMap<u64, Session, S>, D::Error> {
        map.iter()
            .map(|Entry { key, value }| {
                SessionRkyv::deserialize_with(value, d).map(|session| (*key, session))
            })
            .collect()
    }
}

impl<'a, S> SessionsWrapper<'a, S> {
    pub(crate) fn new(sessions: &'a HashMap<u64, Session, S>) -> Self {
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
        C::Error: StdError,
    {
        type Error = CheckOwnedPointerError<[Entry<u64, ArchivedSession>], C>;

        unsafe fn check_bytes<'bytecheck>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'bytecheck Self, Self::Error> {
            ArchivedVec::<Entry<u64, ArchivedSession>>::check_bytes(
                addr_of!((*value).sessions),
                context,
            )?;

            Ok(&*value)
        }
    }
};
