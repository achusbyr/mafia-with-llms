use crate::actor::llm_actor::LlmActor;
use crate::actor::real_actor::RealActor;
use crate::data::action::Action;
use crate::data::extra_data::ExtraData;
use crate::data::roles::GameRole;
use crate::game::Game;
use async_openai::types::chat::ChatCompletionTools;

pub mod llm_actor;
pub mod real_actor;

pub struct BaseActor {
    pub dead: bool,
    pub name: String,
    pub id: u8,
    pub kind: ActorKind,
    pub extra_data: Vec<ExtraData>,
    pub role: GameRole,
}

impl BaseActor {
    pub async fn prompt(&self, prompt: &str, game: &Game, tools: &[ChatCompletionTools]) -> Action {
        match &self.kind {
            ActorKind::Real(real) => real.prompt(prompt, game).await,
            ActorKind::Llm(llm) => {
                llm.ai_interface
                    .send_request_with_tools(prompt, tools)
                    .await
            }
        }
    }
}

pub enum ActorKind {
    Real(RealActor),
    Llm(LlmActor),
}
