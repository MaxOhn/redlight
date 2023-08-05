use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Fallible,
};
use twilight_model::channel::message::sticker::{StickerFormatType, StickerType};

macro_rules! archive_as_u8 {
    ( $ty:ident: $orig:ident ) => {
        impl ArchiveWith<$orig> for $ty {
            type Archived = u8;
            type Resolver = ();

            unsafe fn resolve_with(
                field: &$orig,
                pos: usize,
                resolver: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                u8::resolve(&u8::from(*field), pos, resolver, out);
            }
        }

        impl<S: Fallible + ?Sized> SerializeWith<$orig, S> for $ty {
            fn serialize_with(_: &$orig, _: &mut S) -> Result<(), <S as Fallible>::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> DeserializeWith<u8, $orig, D> for $ty {
            fn deserialize_with(archived: &u8, _: &mut D) -> Result<$orig, <D as Fallible>::Error> {
                Ok($orig::from(*archived))
            }
        }
    };
}

pub struct StickerTypeRkyv;

archive_as_u8!(StickerTypeRkyv: StickerType);

pub struct StickerFormatTypeRkyv;

archive_as_u8!(StickerFormatTypeRkyv: StickerFormatType);
