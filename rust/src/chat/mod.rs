use crate::data::channel::Channel;
use crate::data::context_entry::{ContextEntry, SayerType};
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use godot::classes::{AnimatableBody3D, Camera3D, Control, IControl};
use godot::prelude::*;
use std::collections::HashMap;
use std::f64;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, channel};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub mod ui;
pub mod visuals;

#[derive(GodotClass)]
#[class(base = Control)]
pub struct Chat {
    pub camera: Option<Gd<Camera3D>>,
    pub command_receiver: Receiver<ChatCommand>,
    pub player_nodes: HashMap<u8, Gd<AnimatableBody3D>>,
    pub paused: Arc<AtomicBool>,
    game_iteration: Option<JoinHandle<()>>,
    game: Arc<Mutex<Game>>,
    api_key: GString,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Chat {
    fn init(base: Base<Self::Base>) -> Self {
        let channel = channel::<ChatCommand>();
        let paused = Arc::from(AtomicBool::new(false));
        let paused_clone = Arc::clone(&paused);
        Self {
            camera: None,
            command_receiver: channel.1,
            player_nodes: HashMap::new(),
            paused,
            game_iteration: None,
            game: Arc::from(Mutex::from(Game::new(channel.0, paused_clone))),
            api_key: GString::new(),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        while let Ok(command) = self.command_receiver.try_recv() {
            self.handle_command(command);
        }
        if let Some(game_iteration) = &self.game_iteration {
            if game_iteration.is_finished() {
                self.game_iteration = None;
            } else {
                return;
            }
        }
        let game = Arc::clone(&self.game);
        self.game_iteration = Some(
            crate::tokio::AsyncRuntime::singleton()
                .bind()
                .runtime
                .spawn(async move {
                    let mut game = game.lock().await;
                    if let Some(end_result) = &game.end_result {
                        Game::get_context_mut().push(ContextEntry {
                            content: crate::prompts::general::game_end(end_result),
                            sayer_type: crate::data::context_entry::SayerType::System,
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

    fn enter_tree(&mut self) {
        let api_key_file = godot::classes::FileAccess::open(
            "res://API_KEY.txt",
            godot::classes::file_access::ModeFlags::READ,
        );
        if let Some(file) = api_key_file
            && let text = file.get_as_text()
            && !text.is_empty()
        {
            self.api_key = text.to_string().trim().to_godot();
        } else {
            rfd::MessageDialog::default().set_title("API_KEY.txt not found").set_description("Please put an API_KEY.txt that has the API key in the same directory as the executable").show();
            std::process::exit(1);
        }
        crate::llm::ai_interface::CLIENT.get_or_init(|| {
            Client::with_config(
                OpenAIConfig::default()
                    .with_api_base(crate::llm::ai_interface::API_URL)
                    .with_api_key(&self.api_key),
            )
        });
    }

    fn ready(&mut self) {
        self.camera = Some(
            self.get_world()
                .get_node_as::<godot::classes::Camera3D>("Camera3D"),
        );
        let game = Arc::clone(&self.game);
        crate::tokio::AsyncRuntime::singleton()
            .bind()
            .runtime
            .block_on(async {
                let mut game = game.lock().await;
                game.before_init(self);
                game.init_actors(10, self, None);
                game.init_context(true);
            });
    }
}

impl Chat {
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
