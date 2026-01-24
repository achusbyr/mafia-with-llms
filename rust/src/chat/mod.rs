use crate::data::channel::Channel;
use crate::data::context_entry::{ContextEntry, SayerType};
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use crate::load_message_scene;
use godot::classes::{AnimatableBody3D, Camera3D, Control, IControl};
use godot::prelude::*;
use std::collections::HashMap;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub mod ui;
pub mod visuals;

#[derive(GodotClass)]
#[class(base = Control)]
pub struct Chat {
    pub camera: Option<Gd<Camera3D>>,
    pub command_receiver: Option<Receiver<ChatCommand>>,
    pub player_nodes: HashMap<u8, Gd<AnimatableBody3D>>,
    game_iteration: Option<JoinHandle<()>>,
    game: Option<Arc<Mutex<Game>>>,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Chat {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            camera: None,
            command_receiver: None,
            player_nodes: HashMap::new(),
            game_iteration: None,
            game: None,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        while let Some(command_receiver) = &self.command_receiver
            && let Ok(command) = command_receiver.try_recv()
        {
            self.handle_command(command);
        }
        if let Some(game_iteration) = &self.game_iteration {
            if game_iteration.is_finished() {
                self.game_iteration = None;
            } else {
                return;
            }
        }
        if let Some(game) = &self.game {
            let game = Arc::clone(game);
            self.game_iteration = Some(
                crate::tokio::AsyncRuntime::singleton()
                    .bind()
                    .runtime
                    .spawn(async move {
                        let mut game = game.lock().await;
                        if let Some(end_result) = &game.end_result {
                            Game::get_context_mut().push(ContextEntry {
                                content: crate::prompts::general::game_end(end_result),
                                sayer_type: SayerType::System,
                                extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
                            });
                            game.send_on_behalf_of_chat(ChatCommand::RefreshContextWithActor);
                            // TODO: Post game talk
                            loop {
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
                        }
                        game.iterate().await
                    }),
            )
        }
    }

    fn ready(&mut self) {
        // This whole entire part needs a major cleanup
        self.camera = Some(self.get_world().get_node_as::<Camera3D>("Camera3D"));
        let init_data = unsafe {
            #[allow(static_mut_refs)]
            if let Some(config) = crate::configuration::CONFIGURATION.as_ref() {
                config
            } else {
                panic!("Configuration not initialized");
            }
        };
        let actors = {
            let mut actors = Vec::new();
            for actor in &init_data.3 {
                let base_actor = crate::actor::BaseActor {
                    name: actor.name.clone(),
                    id: actor.id,
                    role: actor.role.clone(),
                    extra_data: actor.extra_data.clone(),
                    kind: match &actor.kind {
                        crate::actor::ActorKind::Real => crate::actor::ActorKind::Real,
                        crate::actor::ActorKind::Llm(ai_interface) => {
                            crate::actor::ActorKind::Llm(crate::llm::ai_interface::AIInterface {
                                model_id: ai_interface.model_id.clone(),
                                owner_id: ai_interface.owner_id,
                            })
                        }
                    },
                    model_customization: actor.model_customization.clone(),
                };
                actors.push(base_actor);
            }
            actors
        };
        self.initialize(init_data.0, init_data.1.clone(), init_data.2, actors);
    }
}

impl Chat {
    pub fn initialize(
        &mut self,
        start_at_night: bool,
        key_url_pair: (String, String),
        playable_actor: Option<u8>,
        actors: Vec<crate::actor::BaseActor>,
    ) {
        crate::llm::ai_interface::CLIENT.get_or_init(|| {
            async_openai::Client::with_config(
                async_openai::config::OpenAIConfig::default()
                    .with_api_base(key_url_pair.1)
                    .with_api_key(key_url_pair.0),
            )
        });
        let channel = channel::<ChatCommand>();
        self.command_receiver = Some(channel.1);
        let mut game = Game::new(channel.0, playable_actor);
        game.before_init(self);
        game.init_actors(actors, self);
        game.init_context(start_at_night);
        self.game = Some(Arc::from(Mutex::from(game)));
    }

    pub fn handle_command(&mut self, command: ChatCommand) {
        match command {
            ChatCommand::Closure(mut closure) => closure(self),
            ChatCommand::CameraFocus(id, content) => {
                self.get_current_text().set_text(&content.to_godot());
                self.focus_camera_on_actor(id);
            }
            ChatCommand::RefreshContextWithActor => {
                let actors = Game::get_actors()
                    .iter()
                    .map(|actor| (actor.name.clone(), actor.id))
                    .collect::<Vec<_>>();
                let context = Game::get_context()
                    .clone()
                    .into_iter()
                    .filter(|entry| {
                        entry.available_for_actor(Game::get_actor_from_id(0).unwrap(), false)
                    })
                    .collect::<Vec<_>>();
                let mut messages = self.get_message_list();
                for mut existing in messages.get_children().iter_shared() {
                    existing.queue_free();
                }
                for entry in &context {
                    let message = load_message_scene().instantiate_as::<Control>();
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
            }
            ChatCommand::RefreshActorList => {
                let mut actor_list = self.get_actor_list();
                for mut existing in actor_list.get_children().iter_shared() {
                    existing.queue_free()
                }
                for actor in Game::get_nondead_actors() {
                    let mut label = godot::classes::Label::new_alloc();
                    label.set_text(&format!("{} (ID {})", actor.name, actor.id));
                    actor_list.add_child(&label);
                }
            }
        }
    }
}

pub enum ChatCommand {
    Closure(Box<dyn FnMut(&mut Chat) + Send>),
    CameraFocus(u8, String),
    RefreshContextWithActor,
    RefreshActorList,
}
