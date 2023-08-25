use twilight_model::{
    gateway::payload::incoming::{
        ReactionAdd, ReactionRemove, ReactionRemoveAll, ReactionRemoveEmoji,
    },
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
};

/// All the different events related to reactions.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ReactionEvent<'a> {
    /// A reaction was added to a message.
    Add(&'a ReactionAdd),
    /// A reaction was removed from a message.
    Remove(&'a ReactionRemove),
    /// All reactions were removed from a message.
    RemoveAll(&'a ReactionRemoveAll),
    /// All reactions with a given emoji of a message were removed.
    RemoveEmoji(&'a ReactionRemoveEmoji),
}

impl ReactionEvent<'_> {
    pub fn message_id(self) -> Id<MessageMarker> {
        match self {
            ReactionEvent::Add(event) => event.message_id,
            ReactionEvent::Remove(event) => event.message_id,
            ReactionEvent::RemoveAll(event) => event.message_id,
            ReactionEvent::RemoveEmoji(event) => event.message_id,
        }
    }

    pub fn channel_id(self) -> Id<ChannelMarker> {
        match self {
            ReactionEvent::Add(event) => event.channel_id,
            ReactionEvent::Remove(event) => event.channel_id,
            ReactionEvent::RemoveAll(event) => event.channel_id,
            ReactionEvent::RemoveEmoji(event) => event.channel_id,
        }
    }
}
