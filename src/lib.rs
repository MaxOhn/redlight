mod cache;
mod error;
mod key;
mod util;
mod value;

/// Types and traits to configure the cache.
pub mod config;

/// Types to help implement rkyv traits.
pub mod rkyv_util;

/// Types and traits related to serialization.
pub mod ser;

#[cfg(feature = "bb8")]
use bb8_redis::redis;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
use deadpool_redis::redis;

pub use self::{cache::RedisCache, error::CacheError, value::CachedValue};

type CacheResult<T> = Result<T, CacheError>;
