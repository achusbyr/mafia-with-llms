use crate::data::action::Action;
use crate::data::extra_data::ExtraData;
use crate::data::roles::GameRole;
use crate::game::Game;
use crate::llm::ai_interface::AIInterface;
use async_openai::types::chat::ChatCompletionTools;
use tokio::sync::mpsc::channel;

pub struct BaseActor {
    pub name: String,
    pub id: u8,
    pub role: GameRole,
    pub extra_data: Vec<ExtraData>,
    pub kind: ActorKind,
    pub model_customization: ModelCustomization,
}

impl BaseActor {
    pub async fn prompt(&self, prompt: &str, game: &Game, tools: &[ChatCompletionTools]) -> Action {
        match &self.kind {
            ActorKind::Real => real_prompt(prompt, game).await,
            ActorKind::Llm(llm) => llm.send_request_with_tools(prompt, tools).await,
        }
    }
}

pub enum ActorKind {
    Real,
    Llm(AIInterface),
}

#[derive(Clone)]
pub struct ModelCustomization {
    pub sprite_path: String,
    pub color: godot::builtin::Color,
}

pub async fn real_prompt(prompt: &str, game: &Game) -> Action {
    let another_prompt = prompt.to_string();
    let prompt = prompt.to_string();
    let (sender, mut receiver) = channel::<Option<Action>>(1);
    game.command_sender
        .send(crate::chat::ChatCommand::Closure(Box::new(move |chat| {
            let sender = sender.clone();
            let message = godot::tools::load::<godot::classes::PackedScene>("res://message.tscn")
                .instantiate_as::<godot::classes::Control>();
            message
                .get_node_as::<godot::classes::Label>("Container/Background/Sayer")
                .set_text("System");
            message
                .get_node_as::<godot::classes::RichTextLabel>("Container/Content")
                .set_text(&prompt);
            chat.get_message_list().add_child(&message);
            let send = chat.get_send_button();
            let mut input = chat.get_input_box();
            godot::task::spawn(async move {
                send.signals().pressed().to_future().await;
                let text = input.get_text();
                input.clear();
                let mut results = Vec::new();
                for line in text.to_string().lines() {
                    if let Some(action) = parse_real_command(line) {
                        results.push(action);
                    }
                }
                if results.is_empty() {
                    sender.send(None).await.unwrap();
                    return;
                }
                if results.len() == 1 {
                    sender.send(Some(results[0].clone())).await.unwrap();
                } else {
                    sender.send(Some(Action::MultiCall(results))).await.unwrap();
                }
            });
        })))
        .unwrap();
    let result = receiver.recv().await.unwrap();
    match result {
        None => Box::pin(real_prompt(&another_prompt, game)).await,
        Some(action) => action,
    }
}

fn parse_real_command(line: &str) -> Option<Action> {
    let line = line.trim();
    if line == "!abstain" {
        Some(Action::Abstain)
    } else if let Some(rest) = line.strip_prefix("!tag ") {
        let id = rest.trim().parse::<u8>().ok()?;
        Some(Action::TagPlayerForComment(id))
    } else if let Some(rest) = line.strip_prefix("!whisper ") {
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let id = parts[0].parse::<u8>().ok()?;
            Some(Action::Whisper(id, parts[1].to_string()))
        } else {
            None
        }
    } else if let Some(rest) = line.strip_prefix("!talk ") {
        Some(Action::Talk(rest.to_string()))
    } else if let Some(rest) = line.strip_prefix("!provide_id ") {
        let id = rest.trim().parse::<u8>().ok()?;
        Some(Action::ProvideID(id))
    } else {
        None
    }
}
