use crate::actor::BaseActor;
use crate::context_entry::{ContextEntry, SayerType};
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::game::{ACTOR_COUNT, EXTRA_MESSAGES, Game};
use crate::llm::tools::Tool;
use crate::prompts::general::{actor_was_killed, day_time, night_time, voting_begins, voting_ends};
use crate::prompts::specific::doctor::{target_protected, you_chose_to_protect};
use crate::prompts::specific::mafia::mafia_discussion_begin;

impl Game {
    pub async fn iterate(&mut self) {
        if self.day_night_count.is_night {
            self.add_to_context(ContextEntry {
                content: night_time(self.day_night_count.night_count),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
            });
        } else {
            if let Some(last_kill) = self.last_kill.take() {
                for actor_id in last_kill {
                    let actor = Self::get_actor_from_id(actor_id).unwrap();
                    self.add_to_context(ContextEntry {
                        content: actor_was_killed(actor),
                        sayer_type: SayerType::System,
                        extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
                    });
                }
            }
            self.add_to_context(ContextEntry {
                content: day_time(self.day_night_count.day_count),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
            });
        }
        self.refresh_actor_list();
        for actor in Self::get_actors_mut() {
            actor.extra_data.clear();
        }
        if self.day_night_count.is_night {
            self.iterate_night().await;
        } else {
            self.iterate_day().await;
        }
    }

    pub async fn iterate_night(&mut self) {
        let actors = Self::get_nondead_actors();
        self.process_sheriff_turn(&actors).await;
        self.process_doctor_turn(&actors).await;
        self.process_mafia_turn().await;
        self.day_night_count.night_count += 1;
        self.day_night_count.is_night = false;
    }

    pub async fn iterate_day(&mut self) {
        self.run_discussion(
            &Self::get_nondead_actors(),
            ACTOR_COUNT,
            EXTRA_MESSAGES,
            vec![ExtraData::SaidInChannel(Channel::Global)],
        )
        .await;
        self.add_to_context(ContextEntry {
            content: voting_begins().to_string(),
            sayer_type: SayerType::System,
            extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
        });
        if let Some(voted_out) = Self::handle_voting(
            &Self::get_nondead_actors(),
            &[ExtraData::SaidInChannel(Channel::Global)],
            self,
        )
        .await
        {
            Self::get_actors_mut()[voted_out as usize].dead = true;
            self.add_to_context(ContextEntry {
                content: voting_ends(Some(Self::get_actor_from_id(voted_out).unwrap()), true),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
            })
        } else {
            self.add_to_context(ContextEntry {
                content: voting_ends(None, false),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
            })
        }
        self.day_night_count.day_count += 1;
        self.day_night_count.is_night = true;
    }
}

impl Game {
    async fn process_sheriff_turn(&mut self, actors: &Vec<&BaseActor>) {
        let sheriffs = actors
            .iter()
            .filter(|a| matches!(a.role, crate::data::roles::GameRole::Sheriff))
            .collect::<Vec<_>>();
        for sheriff in sheriffs {
            let action = sheriff
                .prompt(
                    crate::prompts::specific::sheriff::pick_to_investigate(),
                    self,
                    &[crate::llm::tools::ProvideID::make_tool()],
                )
                .await;
            if let crate::data::action::Action::ProvideID(target_id) = action {
                let target = Self::get_actor_from_id(target_id).unwrap();
                self.add_to_context(ContextEntry {
                    content: crate::prompts::specific::sheriff::investigate_result(target)
                        .to_string(),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(sheriff.id))],
                });
            }
        }
    }

    async fn process_doctor_turn(&mut self, actors: &Vec<&BaseActor>) {
        let doctors = actors
            .iter()
            .filter(|a| matches!(a.role, crate::data::roles::GameRole::Doctor))
            .collect::<Vec<_>>();
        for doctor in doctors {
            let action = doctor
                .prompt(
                    crate::prompts::specific::doctor::pick_to_protect(),
                    self,
                    &[crate::llm::tools::ProvideID::make_tool()],
                )
                .await;
            if let crate::data::action::Action::ProvideID(target_id) = action {
                let target = &mut Self::get_actors_mut()[target_id as usize];
                self.add_to_context(ContextEntry {
                    content: you_chose_to_protect(target),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(doctor.id))],
                });
                target.extra_data.push(ExtraData::ProtectedByDoctor);
            }
        }
    }

    async fn process_mafia_turn(&mut self) {
        let actors = Self::get_nondead_actors();
        let mafias = actors
            .into_iter()
            .filter(|a| matches!(a.role.alignment(), crate::data::roles::RoleAlignment::Mafia))
            .collect::<Vec<_>>();
        self.add_to_context(ContextEntry {
            content: mafia_discussion_begin().to_string(),
            sayer_type: SayerType::System,
            extra_data: vec![ExtraData::SaidInChannel(Channel::Mafia)],
        });
        self.run_discussion(
            &mafias,
            mafias.len() as u8,
            EXTRA_MESSAGES,
            vec![ExtraData::SaidInChannel(Channel::Mafia)],
        )
        .await;
        if let Some(voted_out) =
            Self::handle_voting(&mafias, &[ExtraData::SaidInChannel(Channel::Mafia)], self).await
        {
            let actor = Self::get_actor_from_id(voted_out).unwrap();
            self.add_to_context(ContextEntry {
                content: voting_ends(Some(actor), false),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::Mafia)],
            });
            if actor
                .extra_data
                .iter()
                .any(|data| matches!(data, ExtraData::ProtectedByDoctor))
            {
                self.add_to_context(ContextEntry {
                    content: target_protected(actor),
                    sayer_type: SayerType::System,
                    extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
                });
            } else {
                Self::get_actors_mut()[voted_out as usize].dead = true;
                self.last_kill = Some(vec![voted_out]);
            }
        } else {
            self.add_to_context(ContextEntry {
                content: voting_ends(None, false),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::Mafia)],
            })
        }
    }
}
