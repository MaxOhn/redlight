use rkyv::{
    rancor::{Fallible, Source},
    ser::Writer,
    with::{ArchiveWith, SerializeWith},
    Archive, Deserialize, Place, Serialize,
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
///     #[rkyv(with = IntegrationAccountRkyv)]
///     as_owned: IntegrationAccount,
///     #[rkyv(with = IntegrationAccountRkyv)]
///     as_ref: &'a IntegrationAccount,
/// }
/// ```
#[derive(Archive, Serialize, Deserialize)]
#[rkyv(
    remote = IntegrationAccount,
    archived = ArchivedIntegrationAccount,
    resolver = IntegrationAccountResolver,
    derive(Debug, PartialEq, Eq),
)]
pub struct IntegrationAccountRkyv {
    pub id: String,
    pub name: String,
}

impl From<IntegrationAccountRkyv> for IntegrationAccount {
    fn from(account: IntegrationAccountRkyv) -> Self {
        Self {
            id: account.id,
            name: account.name,
        }
    }
}

impl<S: Fallible<Error: Source> + Writer + ?Sized> SerializeWith<&IntegrationAccount, S>
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

    fn resolve_with(
        field: &&IntegrationAccount,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        <Self as ArchiveWith<IntegrationAccount>>::resolve_with(*field, resolver, out);
    }
}

#[cfg(test)]
mod tests {
    use rkyv::{rancor::Error, with::With};

    use super::*;

    #[test]
    fn test_rkyv_integration_account() -> Result<(), Error> {
        let integration_account = IntegrationAccount {
            id: "id".to_owned(),
            name: "name".to_owned(),
        };

        let bytes = rkyv::to_bytes(With::<_, IntegrationAccountRkyv>::cast(
            &integration_account,
        ))?;

        #[cfg(not(feature = "bytecheck"))]
        let archived = unsafe { rkyv::access_unchecked(&bytes) };

        #[cfg(feature = "bytecheck")]
        let archived = rkyv::access(&bytes)?;

        let deserialized: IntegrationAccount =
            rkyv::deserialize(With::<_, IntegrationAccountRkyv>::cast(archived))?;

        assert_eq!(integration_account, deserialized);

        Ok(())
    }
}
