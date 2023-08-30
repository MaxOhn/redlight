//! A highly performant and customizable third-party redis cache for [twilight].
//!
//! # Usage
//!
//! A comprehensive example can be found in [examples].
//!
//! 1. Create a new config type that implements [`CacheConfig`].
//!     - Create a new type for each associated type you're interested in caching.
//!     - Each associated type must implement its corresponding required traits.
//!     - For associated types you don't want to cache, use [`Ignore`].
//! 2. Create a [`RedisCache`] instance via [`new`] or [`new_with_pool`].
//! 3. In your gateway event loop, pass a reference of the event to [`RedisCache::update`].
//!
//! # What is `rkyv`?
//!
//! In order for data to be stored in redis, it needs a type that redis understands.
//! An obvious choice here is to *serialize* the data into a collection of bytes and then store those bytes.
//! [`serde`] is the most popular crate to handle serialization in combination with implementors such as `serde-json` or `bincode`.
//!
//! Checking out this [benchmark] provides insight to a bunch of options.
//! Most efficient serde-related crates require strict rules for handling data, which twilight's types generally
//! don't satisfy so we won't be able to use crates such as `bincode`, `bare`, `postcard`, etc.
//! Crates such as `flexbuffer`, `capnp`, or `prost` are based on language-agnostic schemas which are
//! way too painful to setup and define so those are no options either.
//! Other crates fall short due to unfitness for production, immaturity, or just insufficient performance.
//!
//! Among the remaining options, [`rkyv`] shines the brightest not only because of its performance and rising popularity,
//! but also because of its key feature: zero-cost-deserialization.
//!
//! [`rkyv`] serializes data in such a way that the written bytes can be re-interpreted as "archived form" without any deserialization at all.
//! This means that whenever we fetch something from the cache, we generally don't need to perform any complex and costly deserialization procedure
//! in order to make sense of the bytes. We just re-interprete them as "archived data" and thus can read fields.
//!
//! As such, `redlight` provides cached data in form of a [`CachedArchive<T>`] instance.
//! [`CachedArchive`] is essentially just a wrapper around some bytes but it also implements [`Deref`] with `Target = Archived<T>`,
//! meaning that you can use it just like you would an archived `T`.
//!
//! # Why use `redlight`?
//!
//! * Pros:
//!     - Data is stored in redis and thus *persistent*, it can remain across reboots. With the `cold_resume` feature there's even a built-in way to resume previous gateway sessions.
//!     - `twilight-cache-inmemory` is required to *own* all its data, meaning it always needs to clone it out of incoming events. `redlight` on the other hand just needs to serialize it which is done via reference.
//!     - The configuration offers a way to cache only the bits and fields that you're interested in instead of the whole thing.
//!     - `redlight` provides redis' built-in expire feature, meaning you can configure cached entries to automatically be removed after a given time.
//!     - There are no ways to deadlock yourself whereas storing data inmemory via `dashmap` hands you a potential foot gun.
//!     - Since the underlying pool is accessible, you technically have full control over all stored data and don't need to rely on the provided API.
//!
//! * Contras:
//!     - `redlight` fully depends on redis. If your connection is slow, so will be the cache.
//!     - All `redlight` cache interactions are async and fallible.
//!     - The configuration requires some setup. Types need to be defined, traits need to be implemented, and [`rkyv`] needs to be utilized which in itself might be a little dawning in the beginning.
//!     - `redlight` comes with a fair bit of unsafe code. Some required due to [`rkyv`]'s unsafe methods, some just to optimize certain operations.
//!
//! # Features
//!
//! | Flag | Description | Dependencies
//! | - | - | -
//! | `default` | Enables the `bb8` and `validation` flag |
//! | `bb8` | Uses [`bb8`] as underlying connection pool | [`bb8-redis`]
//! | `deadpool` | Uses [`deadpool`] as underlying connection pool | [`deadpool-redis`]
//! | `validation` | Always validate data when fetched from the cache. This adds a performance penalty but ensures that stored data always matches the defined types. | `rkyv/validation`
//! | `cold_resume` | Enables the methods `RedisCache::freeze` and `RedisCache::defrost` to store and load discord gateway sessions. | [`twilight-gateway`]
//! | `metrics` | Starts a background task that updates metrics in an interval. Metrics will be recorded in the global recorder which should be set before creating a cache instance. | [`metrics`]
//!
//! Either the `bb8` or `deadpool` feature *must* be enabled.
//!
//! [twilight]: https://github.com/twilight-rs/twilight
//! [examples]: https://github.com/MaxOhn/redlight/tree/main/examples
//! [`CacheConfig`]: https://docs.rs/redlight/latest/redlight/config/trait.CacheConfig.html
//! [`new`]: https://docs.rs/redlight/latest/redlight/struct.RedisCache.html#method.new
//! [`new_with_pool`]: https://docs.rs/redlight/latest/redlight/struct.RedisCache.html#method.new_with_pool
//! [`Ignore`]: https://docs.rs/redlight/latest/redlight/config/struct.Ignore.html
//! [`serde`]: https://docs.rs/serde/latest/serde/
//! [benchmark]: https://github.com/djkoloski/rust_serialization_benchmark#minecraft_savedata
//! [`rkyv`]: https://docs.rs/rkyv/latest/rkyv/
//! [`Deref`]: https://doc.rust-lang.org/std/ops/trait.Deref.html
//! [`bb8`]: https://docs.rs/bb8/latest/bb8/
//! [`bb8-redis`]: https://docs.rs/bb8-redis/latest/bb8_redis/
//! [`deadpool`]: https://docs.rs/deadpool/latest/deadpool/
//! [`deadpool-redis`]: https://docs.rs/deadpool-redis/latest/deadpool_redis/
//! [`twilight-gateway`]: https://docs.rs/twilight-gateway/latest/twilight_gateway/
//! [`metrics`]: https://docs.rs/metrics/latest/metrics/

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links, rustdoc::missing_crate_level_docs)]
#![warn(clippy::missing_const_for_fn, clippy::pedantic)]
#![allow(
    clippy::explicit_iter_loop,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::unused_self
)]

mod cache;
mod key;
mod util;
mod value;

/// Types and traits to configure the cache.
pub mod config;

/// Types related to errors.
pub mod error;

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

pub use self::{cache::RedisCache, error::CacheError, key::RedisKey, value::CachedArchive};

type CacheResult<T> = Result<T, CacheError>;
