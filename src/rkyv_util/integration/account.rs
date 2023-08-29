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
/// use redlight::rkyv_util::integration::IntegrationAccountRkyv;
/// use twilight_model::guild::IntegrationAccount;
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
/// An archived [`IntegrationAccount`].
pub struct ArchivedIntegrationAccount {
    /// The archived counterpart of [`IntegrationAccount::id`].
    pub id: ArchivedString,
    /// The archived counterpart of [`IntegrationAccount::name`].
    pub name: ArchivedString,
}

/// The resolver for an archived [`IntegrationAccount`].
pub struct IntegrationAccountResolver {
    id: StringResolver,
    name: StringResolver,
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

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedIntegrationAccount, IntegrationAccount, D>
    for IntegrationAccountRkyv
{
    fn deserialize_with(
        archived: &ArchivedIntegrationAccount,
        deserializer: &mut D,
    ) -> Result<IntegrationAccount, <D as Fallible>::Error> {
        archived.deserialize(deserializer)
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

impl<D: Fallible + ?Sized> Deserialize<IntegrationAccount, D> for ArchivedIntegrationAccount {
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<IntegrationAccount, <D as Fallible>::Error> {
        Ok(IntegrationAccount {
            id: self.id.deserialize(deserializer)?,
            name: self.name.deserialize(deserializer)?,
        })
    }
}

#[cfg(feature = "validation")]
#[cfg_attr(docsrs, doc(cfg(feature = "validation")))]
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

#[cfg(test)]
mod tests {
    use rkyv::{with::With, Infallible};

    use super::*;

    #[test]
    fn test_rkyv_integration_account() {
        type Wrapper = With<IntegrationAccount, IntegrationAccountRkyv>;

        let integration_account = IntegrationAccount {
            id: "id".to_owned(),
            name: "name".to_owned(),
        };

        let bytes = rkyv::to_bytes::<_, 32>(Wrapper::cast(&integration_account)).unwrap();

        #[cfg(not(feature = "validation"))]
        let archived = unsafe { rkyv::archived_root::<Wrapper>(&bytes) };

        #[cfg(feature = "validation")]
        let archived = rkyv::check_archived_root::<Wrapper>(&bytes).unwrap();

        let deserialized: IntegrationAccount = archived.deserialize(&mut Infallible).unwrap();

        assert_eq!(integration_account, deserialized);
    }
}
