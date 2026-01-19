use crate::actor::BaseActor;
use crate::context_entry::{ContextEntry, SayerType};
use crate::data::action::Action;
use crate::data::channel::Channel;
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
        let mut turn_queue: VecDeque<u8> = actors.iter().map(|b| b.id).collect();

        let mut used_core_message = false;
        let mut used_extra_message = false;

        while let Some(actor_id) = turn_queue.pop_front()
            && core_messages > 0
        {
            if turn_queue.is_empty() && core_messages > 0 {
                let actors: Vec<u8> = Self::get_actors().iter().map(|a| a.id).collect();
                for actor in actors {
                    turn_queue.push_back(actor);
                }
            }

            let action = Self::get_actor_from_id(actor_id)
                .unwrap()
                .prompt(
                    &your_turn_to_talk(core_messages, extra_messages),
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

            self.handle_action(
                actor_id,
                action,
                &mut used_core_message,
                &mut used_extra_message,
                &mut turn_queue,
                &mut extra_data,
            );

            if used_core_message {
                core_messages = core_messages.saturating_sub(1);
            }
            if used_extra_message {
                if extra_messages > 0 {
                    extra_messages = extra_messages.saturating_sub(1);
                } else {
                    core_messages = core_messages.saturating_sub(1);
                }
            }
        }
    }

    fn handle_action(
        &mut self,
        actor_id: u8,
        action: Action,
        used_core_message: &mut bool,
        used_extra_message: &mut bool,
        turn_queue: &mut VecDeque<u8>,
        extra_data: &mut Vec<ExtraData>,
    ) {
        match action {
            Action::Talk(content) => {
                self.add_to_context(ContextEntry {
                    content,
                    sayer_type: SayerType::Actor(actor_id),
                    extra_data: extra_data.clone(),
                });
                *used_core_message = true;
            }
            Action::Abstain => {
                self.add_to_context(ContextEntry {
                    content: abstained_in_discussion(Self::get_actor_from_id(actor_id).unwrap()),
                    sayer_type: SayerType::System,
                    extra_data: extra_data.clone(),
                });
            }
            Action::Whisper { to, message } => {
                let mut extra_data = extra_data.clone();
                extra_data.push(ExtraData::WhisperMetadata { from: actor_id, to });
                self.add_to_context(ContextEntry {
                    content: public_whisper_notice(
                        Self::get_actor_from_id(actor_id).unwrap(),
                        Self::get_actor_from_id(to).unwrap(),
                    ),
                    sayer_type: SayerType::System,
                    extra_data,
                });
                self.add_to_context(ContextEntry {
                    content: whisperer(Self::get_actor_from_id(to).unwrap(), &message),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(actor_id))],
                });
                self.add_to_context(ContextEntry {
                    content: whispered(Self::get_actor_from_id(actor_id).unwrap(), &message),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(to))],
                });
                *used_extra_message = true;
            }
            Action::TagPlayerForComment { id: target_id } => {
                self.add_to_context(ContextEntry {
                    content: tagged_for_comment(
                        Self::get_actor_from_id(actor_id).unwrap(),
                        Self::get_actor_from_id(target_id).unwrap(),
                    ),
                    sayer_type: SayerType::System,
                    extra_data: extra_data.clone(),
                });
                turn_queue.push_front(target_id);
                *used_extra_message = true;
            }
            Action::MultiCall(actions) => {
                for sub_action in actions {
                    self.handle_action(
                        actor_id,
                        sub_action,
                        used_core_message,
                        used_extra_message,
                        turn_queue,
                        extra_data,
                    );
                }
            }
            Action::ProvideID(_) => {
                godot::global::godot_warn!(
                    "{} attempted to use ProvideID",
                    Self::get_actor_from_id(actor_id).unwrap().name
                );
            }
        }
    }
}
