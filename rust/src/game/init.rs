use crate::actor::{ActorKind, BaseActor};
use crate::chat::Chat;
use crate::data::channel::Channel;
use crate::data::context_entry::{ContextEntry, SayerType};
use crate::data::extra_data::ExtraData;
use crate::game::{EXTRA_MESSAGES, Game};
use crate::prompts::general::{build_actor_list, build_role_list, introduce_you, utter_beginning};
use crate::prompts::specific::mafia::build_mafia_list;
use godot::obj::WithBaseField;
use rand::seq::SliceRandom;

impl Game {
    pub fn before_init(&self, chat: &mut Chat) {
        #[cfg(feature = "development")]
        chat.setup_developer_window();
        chat.setup_menu();
    }

    pub fn init_actors(&mut self, actors: Vec<BaseActor>, chat: &mut Chat) {
        Self::get_actors_mut().extend(actors);
        Self::get_actors_mut().shuffle(&mut rand::rng());
        let mut role_pool = Vec::new();
        for _ in 0..3 {
            role_pool.push(crate::data::roles::GameRole::Mafioso);
        }
        role_pool.push(crate::data::roles::GameRole::Doctor);
        role_pool.push(crate::data::roles::GameRole::Sheriff);
        for (index, role) in role_pool
            .iter()
            .enumerate()
            .take(std::cmp::min(Self::get_actors().len(), role_pool.len()))
        {
            Self::get_actors_mut()[index].role = role.clone();
        }
        Self::get_actors_mut().sort_by(|a, b| a.id.cmp(&b.id));
        chat.spawn_visuals(Self::get_actors());
        if Self::get_actors()
            .iter()
            .any(|actor| matches!(actor.kind, ActorKind::Real))
        {
            self.command_sender
                .send(crate::chat::ChatCommand::Closure(Box::new(|chat| {
                    chat.base()
                        .get_node_as::<godot::classes::PanelContainer>("Controls BG")
                        .show();
                })))
                .unwrap();
        }
        for actor in Self::get_actors() {
            let mut output = format!(
                "Init {} for {} (ID {})",
                actor.role.name(),
                actor.name,
                actor.id
            );
            if let ActorKind::Llm(llm) = &actor.kind {
                output.push_str(&format!(" (Model: {})", llm.model_id));
            }
            godot::global::godot_print!("{}", output);
        }
    }

    pub fn init_context(&mut self, start_at_night: bool) {
        self.add_to_context(ContextEntry {
            content: utter_beginning(Self::get_actors().len() as u8, EXTRA_MESSAGES),
            sayer_type: SayerType::System,
            extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
        });
        let mut roles = Self::get_actors()
            .iter()
            .map(|actor| &actor.role)
            .collect::<Vec<_>>();
        roles.sort_by_key(|role| role.name());
        roles.dedup_by(|role, other_role| role.name().eq(&other_role.name()));
        self.add_to_context(ContextEntry {
            content: build_role_list(&roles),
            sayer_type: SayerType::System,
            extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
        });
        self.add_to_context(ContextEntry {
            content: build_actor_list(Self::get_actors()),
            sayer_type: SayerType::System,
            extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
        });
        let mafias = Self::get_actors()
            .iter()
            .filter(|actor| {
                matches!(
                    actor.role.alignment(),
                    crate::data::roles::RoleAlignment::Mafia
                )
            })
            .collect::<Vec<_>>();
        self.add_to_context(ContextEntry {
            content: build_mafia_list(&mafias),
            sayer_type: SayerType::System,
            extra_data: vec![ExtraData::SaidInChannel(Channel::Mafia)],
        });
        for actor in Self::get_actors() {
            self.add_to_context(ContextEntry {
                content: introduce_you(actor),
                sayer_type: SayerType::System,
                extra_data: vec![ExtraData::SaidInChannel(Channel::ToSelf(actor.id))],
            })
        }
        if start_at_night {
            self.day_night_count.is_night = true;
        }
    }
}
