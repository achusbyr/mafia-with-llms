use crate::actor::BaseActor;
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
};

#[derive(Clone)]
pub struct ContextEntry {
    pub content: String,
    pub sayer_type: SayerType,
    pub extra_data: Vec<ExtraData>,
}

impl ContextEntry {
    pub fn to_chat_message(&self, for_actor_id: u8) -> Option<ChatCompletionRequestMessage> {
        let actor = Game::get_actor_from_id(for_actor_id)?;

        if !self.available_for_actor(actor, false) {
            return None;
        }

        match self.sayer_type {
            SayerType::Actor(id) => {
                if id == for_actor_id {
                    Some(
                        ChatCompletionRequestAssistantMessage::from(format!(
                            "{} (ID {}): {}",
                            actor.name, actor.id, self.content
                        ))
                        .into(),
                    )
                } else {
                    Some(
                        ChatCompletionRequestUserMessage {
                            content: format!("{} (ID {}): {}", actor.name, id, self.content).into(),
                            name: Some(actor.name.clone()),
                        }
                        .into(),
                    )
                }
            }
            SayerType::System => {
                Some(ChatCompletionRequestSystemMessage::from(self.content.clone()).into())
            }
        }
    }

    pub fn available_for_actor(&self, actor: &BaseActor, include_raw: bool) -> bool {
        self.extra_data.iter().all(|data| match data {
            ExtraData::WhisperMetadata { from, to } => !(actor.id == *from || actor.id == *to),
            ExtraData::SaidInChannel(channel) => match channel {
                Channel::Global => true,
                Channel::Mafia => matches!(
                    actor.role.alignment(),
                    crate::data::roles::RoleAlignment::Mafia
                ),
                Channel::ToSelf(id) => *id == actor.id,
                Channel::Raw(id) => include_raw && *id == actor.id,
            },
            _ => false,
        })
    }
}

#[derive(Clone)]
pub enum SayerType {
    Actor(u8),
    System,
}
