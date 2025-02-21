pub use validation::{CheckedArchive, CheckedArchived};

#[cfg(feature = "bytecheck")]
mod validation {
    use rkyv::{
        api::high::HighValidator, bytecheck::CheckBytes, rancor::BoxedError, Archive, Portable,
    };

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchive: Archive<Archived: CheckedArchived> {}

    impl<T> CheckedArchive for T where T: Archive<Archived: CheckedArchived> {}

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchived:
        Portable + for<'a> CheckBytes<HighValidator<'a, BoxedError>>
    {
    }

    impl<T> CheckedArchived for T where T: Portable + for<'a> CheckBytes<HighValidator<'a, BoxedError>> {}
}

#[cfg(not(feature = "bytecheck"))]
mod validation {
    use rkyv::{Archive, Portable};

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchive: Archive {}

    impl<T: Archive> CheckedArchive for T {}

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchived: Portable {}

    impl<T: Portable> CheckedArchived for T {}
}
