use twilight_model::channel::message::sticker::{StickerFormatType, StickerType};

pub struct StickerTypeRkyv;

archive_as_u8!(StickerTypeRkyv: StickerType);

pub struct StickerFormatTypeRkyv;

archive_as_u8!(StickerFormatTypeRkyv: StickerFormatType);
