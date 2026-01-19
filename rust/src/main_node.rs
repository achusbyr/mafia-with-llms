use crate::context_entry::ContextEntry;
use crate::data::channel::Channel;
use crate::data::extra_data::ExtraData;
use crate::game::Game;
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use godot::classes::{Button, Control, IControl, TextEdit, VBoxContainer};
use godot::prelude::*;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub type MainCommand = Box<dyn FnMut(&mut Main) + Send>;

#[derive(GodotClass)]
#[class(base = Control)]
pub struct Main {
    #[export]
    api_key: GString,
    game_iteration: Option<JoinHandle<()>>,
    game: Arc<Mutex<Game>>,
    pub command_receiver: Receiver<MainCommand>,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Main {
    fn init(base: Base<Self::Base>) -> Self {
        let channel = channel::<MainCommand>();
        Self {
            api_key: GString::new(),
            game_iteration: None,
            game: Arc::from(Mutex::from(Game::new(channel.0))),
            command_receiver: channel.1,
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
                        game.refresh_context_for_actor(0);
                        loop {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                    game.iterate().await
                }),
        )
    }

    fn enter_tree(&mut self) {
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
                game.init_actors();
                game.init_context(true);
            });
    }
}

impl Main {
    pub fn get_message_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Root UI/Upper/Scroll/Messages")
    }

    pub fn get_actor_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Root UI/Upper/Background/Actors")
    }

    pub fn get_input_box(&self) -> Gd<TextEdit> {
        self.base().get_node_as::<TextEdit>("Root UI/Lower/Input")
    }

    pub fn get_send_button(&self) -> Gd<Button> {
        self.base().get_node_as::<Button>("Root UI/Lower/Send")
    }
}
