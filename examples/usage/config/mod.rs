mod member;
mod role;
mod user;

use redlight::config::{CacheConfig, Ignore};

use self::{member::CachedMember, role::CachedRole, user::CachedUser};

// Implement the trait so we can use it for our cache.
pub struct Config;

// We're only interested in caching members, users, and roles so we define
// types for those and use `Ignore` for everything else.
impl CacheConfig for Config {
    #[cfg(feature = "metrics")]
    // Only if the `metrics` feature is enabled
    const METRICS_INTERVAL_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

    type Channel<'a> = Ignore;
    type CurrentUser<'a> = Ignore;
    type Emoji<'a> = Ignore;
    type Guild<'a> = Ignore;
    type Integration<'a> = Ignore;
    type Member<'a> = CachedMember; // <-
    type Message<'a> = Ignore;
    type Presence<'a> = Ignore;
    type Role<'a> = CachedRole<'a>; // <-
    type ScheduledEvent<'a> = Ignore;
    type StageInstance<'a> = Ignore;
    type Sticker<'a> = Ignore;
    type User<'a> = CachedUser; // <-
    type VoiceState<'a> = Ignore;
}
