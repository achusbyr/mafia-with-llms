use crate::actor::BaseActor;
use crate::chat::ChatCommand;
use crate::data::action::Action;
use crate::data::context_entry::{ContextEntry, SayerType};
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use crate::llm::tools::Tool;
use crate::prompts::general::{actor_voted, time_to_vote};
use std::collections::HashMap;

impl Game {
    pub async fn handle_voting(
        &mut self,
        actors: &[&BaseActor],
        extra_data: &[ExtraData],
    ) -> Option<u8> {
        let mut votes = Vec::new();
        for actor in actors {
            self.check_pause().await;
            let pick = actor
                .prompt(
                    time_to_vote(),
                    self,
                    &[
                        crate::llm::tools::Abstain::make_tool(),
                        crate::llm::tools::Talk::make_tool(),
                        crate::llm::tools::ProvideID::make_tool(),
                        crate::llm::tools::MultiCall::make_tool(),
                    ],
                )
                .await;
            let mut comment: Option<String> = None;
            let mut target_vote: Option<&BaseActor> = None;
            match pick {
                Action::MultiCall(actions) => {
                    for action in actions {
                        if let Action::Talk(message) = action {
                            comment = Some(message);
                        } else if let Action::ProvideID(id) = action {
                            target_vote = Some(Self::get_actor_from_id(id).unwrap());
                            votes.push(id);
                        }
                    }
                }
                Action::ProvideID(id) => {
                    target_vote = Some(Self::get_actor_from_id(id).unwrap());
                    votes.push(id);
                }
                _ => {}
            }
            let text = actor_voted(actor, target_vote, comment);
            self.add_to_context(ContextEntry {
                content: text.clone(),
                sayer_type: SayerType::System,
                extra_data: extra_data.to_vec(),
            });
            if !self.day_night_count.is_night
                || self.playable_actor.is_some()
                    && matches!(
                        Self::get_actor_from_id(self.playable_actor.unwrap())
                            .unwrap()
                            .role
                            .alignment(),
                        crate::data::roles::RoleAlignment::Mafia
                    )
            {
                self.command_sender
                    .send(ChatCommand::CameraFocus(actor.id, text))
                    .unwrap();
            }
        }
        Self::get_voted_out(&votes)
    }

    fn get_voted_out(votes: &[u8]) -> Option<u8> {
        let mut counts = HashMap::new();
        for &id in votes {
            *counts.entry(id).or_insert(0) += 1;
        }

        let max_entry = counts.iter().max_by_key(|&(_, count)| count)?;
        let max_count = *max_entry.1;

        let winners: Vec<u8> = counts
            .into_iter()
            .filter(|&(_, count)| count == max_count)
            .map(|(id, _)| id)
            .collect();

        if winners.len() == 1 {
            Some(winners[0])
        } else {
            None
        }
    }
}
