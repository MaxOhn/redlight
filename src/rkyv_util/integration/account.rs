use rkyv::{
    out_field,
    ser::Serializer,
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Deserialize, Fallible, Serialize,
};
use twilight_model::guild::IntegrationAccount;

/// Used to archive [`IntegrationAccount`].
///
/// # Example
///
/// ```
/// # use rkyv::Archive;
/// use twilight_model::guild::IntegrationAccount;
/// use twilight_redis::rkyv_util::integration::IntegrationAccountRkyv;
///
/// #[derive(Archive)]
/// struct Cached<'a> {
///     #[with(IntegrationAccountRkyv)]
///     as_owned: IntegrationAccount,
///     #[with(IntegrationAccountRkyv)]
///     as_ref: &'a IntegrationAccount,
/// }
/// ```
pub struct IntegrationAccountRkyv;

#[derive(Debug, Eq, PartialEq)]
pub struct ArchivedIntegrationAccount {
    pub id: ArchivedString,
    pub name: ArchivedString,
}

pub struct IntegrationAccountResolver {
    pub id: StringResolver,
    pub name: StringResolver,
}

impl ArchiveWith<IntegrationAccount> for IntegrationAccountRkyv {
    type Archived = ArchivedIntegrationAccount;
    type Resolver = IntegrationAccountResolver;

    unsafe fn resolve_with(
        account: &IntegrationAccount,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.id);
        account.id.resolve(pos + fp, resolver.id, fo);

        let (fp, fo) = out_field!(out.name);
        account.name.resolve(pos + fp, resolver.name, fo);
    }
}

impl ArchiveWith<&IntegrationAccount> for IntegrationAccountRkyv {
    type Archived = <IntegrationAccountRkyv as ArchiveWith<IntegrationAccount>>::Archived;
    type Resolver = <IntegrationAccountRkyv as ArchiveWith<IntegrationAccount>>::Resolver;

    unsafe fn resolve_with(
        field: &&IntegrationAccount,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        <Self as ArchiveWith<IntegrationAccount>>::resolve_with(*field, pos, resolver, out)
    }
}

impl<S: Fallible + Serializer + ?Sized> SerializeWith<IntegrationAccount, S>
    for IntegrationAccountRkyv
{
    fn serialize_with(
        account: &IntegrationAccount,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        Ok(IntegrationAccountResolver {
            id: account.id.serialize(serializer)?,
            name: account.name.serialize(serializer)?,
        })
    }
}

impl<S: Fallible + Serializer + ?Sized> SerializeWith<&IntegrationAccount, S>
    for IntegrationAccountRkyv
{
    fn serialize_with(
        account: &&IntegrationAccount,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        <Self as SerializeWith<IntegrationAccount, S>>::serialize_with(*account, serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedIntegrationAccount, IntegrationAccount, D>
    for IntegrationAccountRkyv
{
    fn deserialize_with(
        archived: &ArchivedIntegrationAccount,
        deserializer: &mut D,
    ) -> Result<IntegrationAccount, <D as Fallible>::Error> {
        Ok(IntegrationAccount {
            id: archived.id.deserialize(deserializer)?,
            name: archived.name.deserialize(deserializer)?,
        })
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use std::ptr::addr_of;

    use rkyv::{
        bytecheck::{ErrorBox, StructCheckError},
        CheckBytes,
    };

    impl<C: ?Sized> CheckBytes<C> for ArchivedIntegrationAccount
    where
        ArchivedString: CheckBytes<C>,
    {
        type Error = StructCheckError;

        unsafe fn check_bytes<'bytecheck>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'bytecheck Self, StructCheckError> {
            <ArchivedString as CheckBytes<C>>::check_bytes(addr_of!((*value).id), context)
                .map_err(|e| StructCheckError {
                    field_name: "id",
                    inner: ErrorBox::new(e),
                })?;

            <ArchivedString as CheckBytes<C>>::check_bytes(addr_of!((*value).name), context)
                .map_err(|e| StructCheckError {
                    field_name: "name",
                    inner: ErrorBox::new(e),
                })?;

            Ok(&*value)
        }
    }
};
