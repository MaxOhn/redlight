pub use validation::CheckedArchive;

#[cfg(feature = "validation")]
mod validation {
    use rkyv::{validation::validators::DefaultValidator, Archive, CheckBytes};

    pub trait CheckedArchive: Archive<Archived = Self::CheckedArchived> {
        type CheckedArchived: for<'a> CheckBytes<DefaultValidator<'a>>;
    }

    impl<T> CheckedArchive for T
    where
        T: Archive,
        <T as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>>,
    {
        type CheckedArchived = <T as Archive>::Archived;
    }
}

#[cfg(not(feature = "validation"))]
mod validation {
    pub trait CheckedArchive {}

    impl<T> CheckedArchive for T {}
}
