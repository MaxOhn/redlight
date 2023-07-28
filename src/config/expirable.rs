/// Provides a way to specify whether cached entries should expire.
pub trait Expirable {
    /// Amount of seconds until a cache entry expires and is removed.
    /// `None` indicates that it will never expire.
    fn expire_seconds() -> Option<usize>;
}
