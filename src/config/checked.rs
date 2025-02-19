pub use validation::CheckedArchive;

#[cfg(feature = "bytecheck")]
mod validation {
    use rkyv::{
        api::high::HighValidator, bytecheck::CheckBytes, rancor::BoxedError, Archive, Archived,
    };

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchive:
        Archive<Archived: for<'a> CheckBytes<HighValidator<'a, BoxedError>>>
    {
    }

    impl<T> CheckedArchive for T
    where
        T: Archive,
        Archived<T>: for<'a> CheckBytes<HighValidator<'a, BoxedError>>,
    {
    }
}

#[cfg(not(feature = "bytecheck"))]
mod validation {
    use rkyv::Archive;

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchive: Archive {}

    impl<T: Archive> CheckedArchive for T {}
}
