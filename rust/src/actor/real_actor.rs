use crate::data::action::Action;
use crate::game::Game;
use tokio::sync::mpsc::channel;

pub struct RealActor;

impl RealActor {
    pub async fn prompt(&self, prompt: &str, game: &Game) -> Action {
        let another_prompt = prompt.to_string();
        let prompt = prompt.to_string();
        let (sender, mut receiver) = channel::<RealPromptResult>(1);
        game.command_sender
            .send(Box::new(move |chat| {
                let sender = sender.clone();
                let message =
                    godot::tools::load::<godot::classes::PackedScene>("res://message.tscn")
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
                        if let Some(action) = Self::parse_command(line) {
                            results.push(action);
                        }
                    }
                    if results.is_empty() {
                        sender.send(RealPromptResult::Nothing).await.unwrap();
                        return;
                    }
                    sender
                        .send(RealPromptResult::Something(Action::MultiCall(results)))
                        .await
                        .unwrap();
                });
            }))
            .unwrap();
        let result = receiver.recv().await.unwrap();
        match result {
            RealPromptResult::Nothing => Box::pin(self.prompt(&another_prompt, game)).await,
            RealPromptResult::Something(action) => action,
        }
    }

    fn parse_command(line: &str) -> Option<Action> {
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
}

enum RealPromptResult {
    Nothing,
    Something(Action),
}
