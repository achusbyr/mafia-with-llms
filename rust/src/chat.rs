use crate::actor::BaseActor;
use crate::context_entry::ContextEntry;
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use godot::classes::{
    AnimatableBody3D, Button, Camera3D, Control, IControl, Label, MeshInstance3D, TextEdit,
    VBoxContainer, Window,
};
use godot::prelude::*;
use std::collections::HashMap;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub type ChatCommand = Box<dyn FnMut(&mut Chat) + Send>;

#[derive(GodotClass)]
#[class(base = Control)]
pub struct Chat {
    #[export]
    pub camera: OnEditor<Gd<Camera3D>>,
    pub command_receiver: Receiver<ChatCommand>,
    pub player_nodes: HashMap<u8, Gd<AnimatableBody3D>>,
    game_iteration: Option<JoinHandle<()>>,
    game: Arc<Mutex<Game>>,
    api_key: GString,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Chat {
    fn init(base: Base<Self::Base>) -> Self {
        let channel = channel::<ChatCommand>();
        Self {
            camera: OnEditor::default(),
            command_receiver: channel.1,
            player_nodes: HashMap::new(),
            game_iteration: None,
            game: Arc::from(Mutex::from(Game::new(channel.0))),
            api_key: GString::new(),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        while let Ok(mut command) = self.command_receiver.try_recv() {
            command(self);
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
                            sayer_type: crate::context_entry::SayerType::System,
                            extra_data: vec![ExtraData::SaidInChannel(Channel::Global)],
                        });
                        game.refresh_context_with_actor(0);
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
            match input_dialogs::Input::new("Enter your API key for the API (OpenRouter/etc.)")
                .show()
            {
                Ok(api_key) => self.api_key = api_key.unwrap().to_godot(),
                Err(err) => {
                    godot::global::godot_print!("Error: {}", err);
                    self.base().get_tree().unwrap().quit();
                }
            }
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
        let game = Arc::clone(&self.game);
        crate::tokio::AsyncRuntime::singleton()
            .bind()
            .runtime
            .block_on(async {
                let mut game = game.lock().await;
                game.before_init();
                game.init_actors();
                game.init_context(true);
            });
    }
}

impl Chat {
    fn spawn_visuals(&mut self, actors: &[BaseActor]) {
        let town_center_pos = self.get_town_center().get_global_position();
        let count = actors.len() as f64;
        let radius = 2.0;

        for (index, actor) in actors.iter().enumerate() {
            let mut instance =
                load::<PackedScene>("res://models/model.tscn").instantiate_as::<AnimatableBody3D>();
            let angle = 2.0 * f64::consts::PI / count * (index as f64);
            let offset = Vector3::FORWARD.rotated(Vector3::UP, angle as real) * radius;
            let final_pos = town_center_pos + offset;
            instance.set_global_position(final_pos);
            instance.look_at(town_center_pos);

            // Set Name/Text (Assuming your player scene has a method or Label)
            // You might need to expose a set_text method on the GDScript attached to the player scene
            // or get the node manually:
            // instance.get_node_as::<Label>("NameLabel").set_text(&actor.name);

            self.base_mut()
                .add_child(&instance.clone().upcast::<Node>());

            self.player_nodes.insert(actor.id, instance);

            /*if matches!(actor.role, crate::data::roles::GameRole::Mafioso) {
                 let mut clone = self.player_scene.instantiate_as::<Node3D>();
                 // Position relative to mafia spawn...
                 let mafia_pos = self.mafia_spawn.get_global_position();
                 // Simple offset logic for clones
                 let clone_offset = Vector3::new(i as real * 1.0, 0.0, 0.0);
                 clone.set_global_position(mafia_pos + clone_offset);
                 self.base_mut().add_child(clone.upcast());
            }*/
        }
    }

    fn get_world(&self) -> Gd<Node3D> {
        self.base().get_node_as::<Node3D>("..")
    }

    pub fn get_town_center(&self) -> Gd<MeshInstance3D> {
        self.get_world()
            .get_node_as::<MeshInstance3D>("Town Center")
    }

    pub fn get_message_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Msg BG/Scroll/Messages")
    }

    pub fn get_actor_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Actors BG/Scroll/Actors")
    }

    pub fn get_input_box(&self) -> Gd<TextEdit> {
        self.base()
            .get_node_as::<TextEdit>("Controls BG/Controls/Input")
    }

    pub fn get_send_button(&self) -> Gd<Button> {
        self.base()
            .get_node_as::<Button>("Controls BG/Controls/Send")
    }

    pub fn get_menu_button(&self) -> Gd<Button> {
        self.base().get_node_as::<Button>("MenuButton")
    }

    pub fn get_menu(&self) -> Gd<Control> {
        self.base().get_node_as::<Control>("Menu")
    }

    pub fn get_current_text(&self) -> Gd<Label> {
        self.base().get_node_as::<Label>("Text BG/Current Text")
    }

    #[cfg(feature = "development")]
    pub fn get_development_window(&self) -> Gd<Window> {
        self.base().get_node_as::<Window>("Developer Window")
    }
}
