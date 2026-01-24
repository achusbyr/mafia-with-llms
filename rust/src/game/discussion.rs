use crate::actor::BaseActor;
use crate::chat::ChatCommand;
use crate::data::action::Action;
use crate::data::channel::Channel;
use crate::data::context_entry::{ContextEntry, SayerType};
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use crate::llm::tools::Tool;
use crate::prompts::general::{
    abstained_in_discussion, public_whisper_notice, tagged_for_comment, whispered, whisperer,
    your_turn_to_talk,
};
use std::collections::VecDeque;

impl Game {
    pub async fn run_discussion(
        &mut self,
        actors: &[&BaseActor],
        mut core_messages: u8,
        mut extra_messages: u8,
        mut extra_data: Vec<ExtraData>,
    ) {
        let mut turn_queue: VecDeque<u8> = actors.iter().map(|actor| actor.id).collect();

        let mut used_message = false;

        while let Some(actor_id) = turn_queue.pop_front()
            && core_messages > 0
        {
            self.check_pause().await;

            if turn_queue.is_empty() && core_messages > 0 {
                actors
                    .iter()
                    .map(|actor| actor.id)
                    .for_each(|item| turn_queue.push_back(item));
            }

            let actor = Self::get_actor_from_id(actor_id).unwrap();

            let action = actor
                .prompt(
                    &your_turn_to_talk(actor, core_messages, extra_messages),
                    self,
                    &[
                        crate::llm::tools::Abstain::make_tool(),
                        crate::llm::tools::Talk::make_tool(),
                        crate::llm::tools::TagPlayerForComment::make_tool(),
                        crate::llm::tools::Whisper::make_tool(),
                        crate::llm::tools::MultiCall::make_tool(),
                    ],
                )
                .await;

            self.handle_and_focus(
                actor_id,
                action,
                &mut used_message,
                &mut turn_queue,
                &mut extra_data,
            );

            godot::global::godot_print!(
                "Turn: {} (Before handling action) Core messages: {} Extra messages: {}",
                actor.name,
                core_messages,
                extra_messages
            );

            if used_message {
                if extra_messages > 0 {
                    extra_messages = extra_messages.saturating_sub(1);
                } else {
                    core_messages = core_messages.saturating_sub(1);
                }
            }

            godot::global::godot_print!(
                "Turn: {} (After handling action) Core messages: {} Extra messages: {}",
                actor.name,
                core_messages,
                extra_messages
            );
        }
    }

    fn handle_and_focus(
        &mut self,
        actor_id: u8,
        action: Action,
        used_message: &mut bool,
        turn_queue: &mut VecDeque<u8>,
        extra_data: &mut Vec<ExtraData>,
    ) {
        let mut final_content = String::new();
        self.handle_action(
            actor_id,
            action,
            used_message,
            turn_queue,
            extra_data,
            &mut final_content,
        );
        if !final_content.is_empty() {
            self.command_sender
                .send(ChatCommand::CameraFocus(
                    actor_id,
                    final_content.trim().to_string(),
                ))
                .unwrap();
        }
    }

    fn handle_action(
        &mut self,
        actor_id: u8,
        action: Action,
        used_message: &mut bool,
        turn_queue: &mut VecDeque<u8>,
        extra_data: &mut Vec<ExtraData>,
        final_content: &mut String,
    ) {
        match action {
            Action::Talk(content) => {
                self.add_to_context(ContextEntry {
                    content: content.clone(),
                    sayer_type: SayerType::Actor(actor_id),
                    extra_data: extra_data.clone(),
                });
                *used_message = true;
                final_content.push_str(&format!("{}\n", &content));
            }
            Action::Abstain => {
                self.add_to_context(ContextEntry {
                    content: abstained_in_discussion(Self::get_actor_from_id(actor_id).unwrap()),
                    sayer_type: SayerType::System,
                    extra_data: extra_data.clone(),
                });
                final_content.push_str("*Abstained*\n");
            }
            Action::Whisper(to, message) => {
                let mut extra_data = extra_data.clone();
                extra_data.push(ExtraData::WhisperMetadata { from: actor_id, to });
                let from = Self::get_actor_from_id(actor_id).unwrap();
                let target = Self::get_actor_from_id(to).unwrap();
                self.add_to_context(ContextEntry {
                    content: public_whisper_notice(from, Self::get_actor_from_id(to).unwrap()),
                    sayer_type: SayerType::System,
                    extra_data,
                });
                self.add_to_context(ContextEntry {
                    content: whisperer(target, &message),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(actor_id))],
                });
                self.add_to_context(ContextEntry {
                    content: whispered(from, &message),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(to))],
                });
                *used_message = true;
                final_content.push_str(&format!("*Whispered to {}*\n", target.name));
            }
            Action::TagPlayerForComment(target_id) => {
                let target = Self::get_actor_from_id(target_id).unwrap();
                self.add_to_context(ContextEntry {
                    content: tagged_for_comment(Self::get_actor_from_id(actor_id).unwrap(), target),
                    sayer_type: SayerType::System,
                    extra_data: extra_data.clone(),
                });
                turn_queue.push_front(target_id);
                *used_message = true;
                final_content.push_str(&format!("*Tagged {}*\n", target.name));
            }
            Action::MultiCall(actions) => {
                for sub_action in actions {
                    self.handle_action(
                        actor_id,
                        sub_action,
                        used_message,
                        turn_queue,
                        extra_data,
                        final_content,
                    );
                }
            }
            Action::ProvideID(_) => {
                godot::global::godot_warn!(
                    "{} attempted to use ProvideID in a discussion",
                    Self::get_actor_from_id(actor_id).unwrap().name
                );
            }
        }
    }
}
