use twilight_model::channel::message::MessageType;

pub struct MessageTypeRkyv;

archive_as_u8!(MessageTypeRkyv: MessageType);
