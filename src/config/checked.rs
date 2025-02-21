pub use validation::CheckedArchived;

#[cfg(feature = "bytecheck")]
mod validation {
    use rkyv::{api::high::HighValidator, bytecheck::CheckBytes, rancor::BoxedError, Portable};

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
    use rkyv::Portable;

    /// Auxiliary trait ensuring properties related to the `bytecheck` feature
    /// flag.
    ///
    /// Automatically implemented for all appropriate types.
    pub trait CheckedArchived: Portable {}

    impl<T: Portable> CheckedArchived for T {}
}
