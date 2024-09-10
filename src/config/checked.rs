pub use validation::CheckedArchive;

#[cfg(feature = "bytecheck")]
mod validation {
    use rkyv::{
        api::high::HighValidator, bytecheck::CheckBytes, rancor::Fallible, Archive, Archived,
    };

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchive<E = <Self as Fallible>::Error>:
        Archive<Archived: for<'a> CheckBytes<HighValidator<'a, E>>>
    {
    }

    impl<T, E> CheckedArchive<E> for T
    where
        T: Archive,
        Archived<T>: for<'a> CheckBytes<HighValidator<'a, E>>,
    {
    }
}

#[cfg(not(feature = "bytecheck"))]
mod validation {
    use rkyv::{rancor::Fallible, Archive};

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchive<E = <Self as Fallible>::Error>: Archive {}

    impl<T: Archive, E> CheckedArchive<E> for T {}
}
