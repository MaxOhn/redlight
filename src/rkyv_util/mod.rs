macro_rules! archive_as_u8 {
    ( $ty:ident: $orig:ident ) => {
        impl rkyv::with::ArchiveWith<$orig> for $ty {
            type Archived = u8;
            type Resolver = ();

            unsafe fn resolve_with(
                field: &$orig,
                pos: usize,
                resolver: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                <u8 as rkyv::Archive>::resolve(&u8::from(*field), pos, resolver, out);
            }
        }

        impl<S: rkyv::Fallible + ?Sized> rkyv::with::SerializeWith<$orig, S> for $ty {
            fn serialize_with(_: &$orig, _: &mut S) -> Result<(), <S as rkyv::Fallible>::Error> {
                Ok(())
            }
        }

        impl<D: rkyv::Fallible + ?Sized> rkyv::with::DeserializeWith<u8, $orig, D> for $ty {
            fn deserialize_with(
                archived: &u8,
                _: &mut D,
            ) -> Result<$orig, <D as rkyv::Fallible>::Error> {
                Ok($orig::from(*archived))
            }
        }
    };
}

pub mod id;
pub mod integration;
pub mod sticker;
pub mod util;
