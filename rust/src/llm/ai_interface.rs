use crate::context_entry::{ContextEntry, SayerType};
use crate::data::action::Action;
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use crate::llm::OpenRouterResponse;
use crate::llm::tools::{MultiCall, ProvideID, TagPlayerForComment, Talk, Whisper};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessage, ChatCompletionTools, CreateChatCompletionRequestArgs,
    ReasoningEffort,
};
use std::sync::OnceLock;

pub const API_URL: &str = "https://openrouter.ai/api/v1";
pub static CLIENT: OnceLock<Client<OpenAIConfig>> = OnceLock::new();

pub struct AIInterface {
    pub model_id: String,
    pub owner_id: u8,
}

impl AIInterface {
    pub async fn send_request_with_tools(
        &self,
        prompt: &str,
        tools: &[ChatCompletionTools],
    ) -> Action {
        let mut messages: Vec<_> = Game::get_context()
            .iter()
            .filter_map(|entry| entry.to_chat_message(self.owner_id))
            .collect();
        messages.push(ChatCompletionRequestSystemMessage::from(prompt).into());
        let client = CLIENT.get().unwrap();
        let request = CreateChatCompletionRequestArgs::default()
            .messages(messages)
            .tools(tools)
            .model(&self.model_id)
            .reasoning_effort(ReasoningEffort::Minimal)
            .build()
            .unwrap();
        let response: OpenRouterResponse = client.chat().create_byot(&request).await.unwrap();

        if let Some(choice) = response.choices.first() {
            let mut collected_actions = Vec::new();

            {
                let mut context_entry: ContextEntry = ContextEntry {
                    content: String::from("NONE"),
                    sayer_type: SayerType::Actor(self.owner_id),
                    extra_data: vec![ExtraData::SaidInChannel(Channel::Raw(self.owner_id))],
                };
                if let Some(content) = &choice.message.content
                    && !content.is_empty()
                {
                    context_entry.content = format!("CONTENT:\n{}\n", content.trim());
                }
                if let Some(reasoning) = &choice.message.reasoning
                    && !reasoning.is_empty()
                {
                    context_entry.content = format!("REASONING:\n{}\n", reasoning.trim(),);
                }
                context_entry.content = context_entry.content.trim().to_string();
                if context_entry.content != "NONE" {
                    Game::get_context_mut().push(context_entry);
                }
            }

            return if let Some(tool_calls) = &choice.message.tool_calls {
                for tool_call in tool_calls {
                    handle_tool_call(
                        (
                            &tool_call.function.name,
                            tool_call.function.arguments.clone(),
                        ),
                        &mut collected_actions,
                    );
                }

                if collected_actions.len() == 1 {
                    collected_actions.into_iter().next().unwrap()
                } else {
                    Action::MultiCall(collected_actions)
                }
            } else {
                godot::global::godot_warn!("No tool was used, abstaining");
                Action::Abstain
            };
        }

        unreachable!();
    }
}

fn handle_tool_call(tool_call: (&str, String), collected_actions: &mut Vec<Action>) {
    match tool_call.0 {
        "Abstain" => collected_actions.push(Action::Abstain),
        "Whisper" => {
            let whisper = serde_json::from_str::<Whisper>(&tool_call.1).unwrap();
            collected_actions.push(Action::Whisper(whisper.to, whisper.message));
        }
        "TagPlayerForComment" => {
            collected_actions.push(Action::TagPlayerForComment(
                serde_json::from_str::<TagPlayerForComment>(&tool_call.1)
                    .unwrap()
                    .id,
            ));
        }
        "ProvideID" => collected_actions.push(Action::ProvideID(
            serde_json::from_str::<ProvideID>(&tool_call.1).unwrap().id,
        )),
        "Talk" => collected_actions.push(Action::Talk(
            serde_json::from_str::<Talk>(&tool_call.1).unwrap().message,
        )),
        "MultiCall" => {
            let multi_call = serde_json::from_str::<MultiCall>(&tool_call.1).unwrap();
            for action in &multi_call.actions {
                handle_tool_call(
                    (&action.tool, action.arguments.to_string()),
                    collected_actions,
                );
            }
        }
        _ => panic!("Unknown tool call"),
    }
}
