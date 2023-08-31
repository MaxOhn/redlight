use std::error::Error as StdError;

use rkyv::{ser::Serializer, Fallible};

/// Auxiliary trait to circumvent the fact that rust currently won't let
/// you specify trait bounds on associated types within trait bounds.
///
/// This trait is implemented automatically for all appropriate types.
pub trait SerializerExt: Serializer<Error = Self::ErrorExt> {
    type ErrorExt: StdError + Send + Sync + 'static;
}

impl<T> SerializerExt for T
where
    T: Serializer,
    <T as Fallible>::Error: StdError + Send + Sync,
{
    type ErrorExt = <Self as Fallible>::Error;
}
