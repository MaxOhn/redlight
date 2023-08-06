mod account;
mod kind;

use twilight_model::guild::IntegrationExpireBehavior;

pub use self::{
    account::{ArchivedIntegrationAccount, IntegrationAccountResolver, IntegrationAccountRkyv},
    kind::GuildIntegrationTypeRkyv,
};

pub struct IntegrationExpireBehaviorRkyv;

archive_as_u8!(IntegrationExpireBehaviorRkyv: IntegrationExpireBehavior);
