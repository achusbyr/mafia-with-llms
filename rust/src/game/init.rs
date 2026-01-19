use crate::actor::llm_actor::LlmActor;
use crate::actor::real_actor::RealActor;
use crate::actor::{ActorKind, BaseActor};
use crate::context_entry::{ContextEntry, SayerType};
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::data::roles::GameRole;
use crate::game::{ACTOR_COUNT, EXTRA_MESSAGES, Game};
use crate::llm::ai_interface::AIInterface;
use crate::llm::model_pool::take_random_model;
use crate::prompts::general::{build_actor_list, build_role_list, introduce_you, utter_beginning};
use crate::prompts::specific::mafia::build_mafia_list;
use godot::obj::{NewAlloc, WithBaseField};
use rand::seq::SliceRandom;

impl Game {
    pub fn init_actors(&mut self) {
        #[cfg(feature = "development")]
        self.setup_developer_window();
        let actor = BaseActor {
            dead: false,
            name: "Player".to_string(),
            id: 0,
            extra_data: Vec::new(),
            kind: ActorKind::Real(RealActor {}),
            role: GameRole::Villager,
        };
        Self::get_actors_mut().push(actor);
        for index in 1..ACTOR_COUNT {
            let model = take_random_model();
            let actor = BaseActor {
                dead: false,
                name: model.display_name,
                id: index,
                extra_data: Vec::new(),
                kind: ActorKind::Llm(LlmActor {
                    ai_interface: AIInterface {
                        model_id: model.model_id,
                        owner_id: index,
                    },
                }),
                role: GameRole::Villager,
            };
            Self::get_actors_mut().push(actor);
        }
        Self::get_actors_mut().shuffle(&mut rand::rng());
        let mut role_pool = Vec::new();
        for _ in 0..3 {
            role_pool.push(GameRole::Mafioso);
        }
        role_pool.push(GameRole::Doctor);
        role_pool.push(GameRole::Sheriff);
        for (index, role) in role_pool
            .iter()
            .enumerate()
            .take(std::cmp::min(Self::get_actors().len(), role_pool.len()))
        {
            Self::get_actors_mut()[index].role = role.clone();
        }
        Self::get_actors_mut().sort_by(|a, b| a.id.cmp(&b.id));
        for actor in Self::get_actors() {
            let mut output = format!(
                "Init {} for {} (ID {})",
                actor.role.name(),
                actor.name,
                actor.id
            );
            if let ActorKind::Llm(llm) = &actor.kind {
                output.push_str(&format!(" (Model: {})", llm.ai_interface.model_id));
            }
            godot::global::godot_print!("{}", output);
        }
    }

    pub fn init_context(&mut self, start_at_night: bool) {
        self.add_to_context(ContextEntry {
            content: utter_beginning(ACTOR_COUNT, EXTRA_MESSAGES),
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

    pub fn refresh_actor_list(&self) {
        let actors = Self::get_nondead_actors()
            .iter()
            .map(|actor| (actor.name.clone(), actor.id))
            .collect::<Vec<_>>();
        self.command_sender
            .send(Box::new(move |main| {
                let mut actor_list = main.get_actor_list();
                for mut existing in actor_list.get_children().iter_shared() {
                    existing.queue_free()
                }
                for actor in &actors {
                    let mut label = godot::classes::Label::new_alloc();
                    label.set_text(&format!("{} (ID {})", actor.0, actor.1));
                    actor_list.add_child(&label);
                }
            }))
            .unwrap();
    }

    pub fn refresh_context_for_actor(&self, id: u8) {
        let actors = Self::get_actors()
            .iter()
            .map(|actor| (actor.name.clone(), actor.id))
            .collect::<Vec<_>>();
        let context = Self::get_context()
            .clone()
            .into_iter()
            .filter(|entry| entry.available_for_actor(Self::get_actor_from_id(id).unwrap(), false))
            .collect::<Vec<_>>();
        self.command_sender
            .send(Box::new(move |main| {
                let mut messages = main.get_message_list();
                for mut existing in messages.get_children().iter_shared() {
                    existing.queue_free();
                }
                let message_scene =
                    godot::tools::load::<godot::classes::PackedScene>("res://message.tscn");
                for entry in &context {
                    let message = message_scene.instantiate_as::<godot::classes::Control>();
                    match entry.sayer_type {
                        SayerType::Actor(id) => {
                            let actor = actors.iter().find(|actor| actor.1 == id).unwrap();
                            message
                                .get_node_as::<godot::classes::Label>("Container/Background/Sayer")
                                .set_text(&format!("{} (ID {})", actor.0, actor.1));
                        }
                        SayerType::System => {
                            message
                                .get_node_as::<godot::classes::Label>("Container/Background/Sayer")
                                .set_text("System");
                        }
                    }
                    message
                        .get_node_as::<godot::classes::RichTextLabel>("Container/Content")
                        .set_text(&entry.content);
                    messages.add_child(&message);
                }
            }))
            .unwrap();
    }

    fn setup_developer_window(&self) {
        self.command_sender
            .send(Box::new(|main| {
                let mut viewport = main.base().get_viewport().unwrap();
                viewport.set_embedding_subwindows(false);
                let mut developer_window = main
                    .base()
                    .get_node_as::<godot::classes::Window>("DeveloperWindow");
                developer_window
                    .signals()
                    .close_requested()
                    .connect_self(|window| {
                        window.hide();
                    });
                developer_window.set_title("Developer Window");
                developer_window.set_size(viewport.get_visible_rect().size.cast_int());
                developer_window.set_visible(true);
                let developer =
                    developer_window.get_node_as::<godot::classes::Control>("Developer");
                let id_select = developer
                    .get_node_as::<godot::classes::OptionButton>("Root UI/Control Panel/ID Select");
                id_select.signals().pressed().connect_self(|button| {
                    button.clear();
                    for actor in Self::get_actors() {
                        button.add_item(&format!("{} (ID {})", actor.name, actor.id));
                    }
                });
                developer
                    .get_node_as::<godot::classes::Button>("Root UI/Control Panel/Build")
                    .signals()
                    .pressed()
                    .connect(move || {
                        let context = Self::get_context()
                            .clone()
                            .into_iter()
                            .filter(|entry| {
                                entry.available_for_actor(
                                    Self::get_actor_from_id(id_select.get_selected_id() as u8)
                                        .unwrap(),
                                    developer
                                        .get_node_as::<godot::classes::CheckBox>(
                                            "Root UI/Control Panel/Include Raw",
                                        )
                                        .is_pressed(),
                                )
                            })
                            .collect::<Vec<_>>();
                        let mut messages = developer.get_node_as::<godot::classes::VBoxContainer>(
                            "Root UI/Scroll/Messages",
                        );
                        for mut existing in messages.get_children().iter_shared() {
                            existing.queue_free();
                        }
                        let message_scene =
                            godot::tools::load::<godot::classes::PackedScene>("res://message.tscn");
                        for entry in context {
                            let message = message_scene.instantiate_as::<godot::classes::Control>();
                            match entry.sayer_type {
                                SayerType::Actor(id) => {
                                    let actor = Self::get_actors()
                                        .iter()
                                        .find(|actor| actor.id == id)
                                        .unwrap();
                                    message
                                        .get_node_as::<godot::classes::Label>(
                                            "Container/Background/Sayer",
                                        )
                                        .set_text(&format!("{} (ID {})", actor.name, actor.id));
                                }
                                SayerType::System => {
                                    message
                                        .get_node_as::<godot::classes::Label>(
                                            "Container/Background/Sayer",
                                        )
                                        .set_text("System");
                                }
                            }
                            message
                                .get_node_as::<godot::classes::RichTextLabel>("Container/Content")
                                .set_text(&entry.content);
                            messages.add_child(&message);
                        }
                    });
            }))
            .unwrap();
    }
}
