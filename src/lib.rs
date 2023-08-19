mod cache;
mod error;
mod key;
mod util;
mod value;

/// Types and traits to configure the cache.
pub mod config;

/// Types related to iteration of cache entries.
pub mod iter;

/// Types to help implement rkyv traits.
pub mod rkyv_util;

/// Types and traits related to serialization.
pub mod ser;

/// Types related to statistics of the cache.
pub mod stats;

/// Re-export of redis types and traits.
pub(crate) mod redis;

pub use self::{cache::RedisCache, error::CacheError, value::CachedArchive};

type CacheResult<T> = Result<T, CacheError>;
