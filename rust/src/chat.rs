use crate::actor::BaseActor;
use crate::context_entry::ContextEntry;
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use godot::classes::{
    AnimatableBody3D, Button, Camera3D, Control, IControl, Label, Label3D, MeshInstance3D,
    TextEdit, VBoxContainer, Window,
};
use godot::prelude::*;
use std::collections::HashMap;
use std::f64;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, channel};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub type ChatCommand = Box<dyn FnMut(&mut Chat) + Send>;

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
                        game.refresh_context_with_actor();
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
                game.before_init();
                game.init_actors(self);
                game.init_context(false);
            });
    }
}

impl Chat {
    pub fn focus_camera_on_actor(&mut self, actor_id: u8, is_night: bool) {
        if is_night {
        } else if let Some(target_node) = self.player_nodes.get(&actor_id) {
            let target_pos = target_node.get_global_position();
            let camera = self.camera.clone().unwrap();
            if let Some(mut tween) = self.base_mut().create_tween() {
                let mut temp_cam = camera.clone();
                temp_cam.look_at(target_pos);
                let target_rot = temp_cam.get_global_rotation();
                tween.tween_property(
                    &camera.upcast::<godot::classes::Object>(),
                    "global_rotation",
                    &target_rot.to_variant(),
                    5.0,
                );
            }
        }
    }

    pub fn spawn_visuals(&mut self, actors: &[BaseActor]) {
        let town_center_pos = self.get_town_center().get_global_position();
        let count = actors.len() as f64;
        let radius = 2.0;

        for (index, actor) in actors.iter().enumerate() {
            let mut instance =
                load::<PackedScene>("res://models/model.tscn").instantiate_as::<AnimatableBody3D>();
            let angle = 2.0 * f64::consts::PI / count * (index as f64);
            let offset = Vector3::FORWARD.rotated(Vector3::UP, angle as real) * radius;
            let final_pos = town_center_pos + offset;

            self.get_world()
                .call_deferred("add_child", &[instance.clone().to_variant()]);

            instance.look_at_from_position(final_pos, town_center_pos);
            instance.translate_object_local(Vector3 {
                x: 0.0,
                y: 0.75,
                z: 0.0,
            }); // Slightly above ground
            instance.rotate_object_local(Vector3::UP, std::f64::consts::PI as real); // Spin 180
            instance
                .get_node_as::<Label3D>("Name")
                .set_text(&actor.name);
            instance
                .get_node_as::<Label3D>("Role")
                .set_text(&actor.role.name());

            self.player_nodes.insert(actor.id, instance);
        }
    }

    #[cfg(feature = "development")]
    pub fn get_development_window(&self) -> Gd<Window> {
        self.base().get_node_as::<Window>("Developer Window")
    }

    pub fn get_message_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Messages Window/Msg BG/Scroll/Messages")
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

    pub fn get_town_center(&self) -> Gd<MeshInstance3D> {
        self.get_world()
            .get_node_as::<MeshInstance3D>("Town Center")
    }

    fn get_world(&self) -> Gd<Node3D> {
        self.base().get_node_as::<Node3D>("../..")
    }
}
