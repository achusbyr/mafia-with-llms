mod access;
mod discussion;
mod init;
mod iterate;
mod voting;

use crate::actor::BaseActor;
use crate::chat::ChatCommand;
use crate::context_entry::ContextEntry;
use crate::data::roles::RoleAlignment;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

const ACTOR_COUNT: u8 = 10;
const EXTRA_MESSAGES: u8 = 5;

static mut ACTORS: Vec<BaseActor> = Vec::new();
static mut CONTEXT: Vec<ContextEntry> = Vec::new();

pub struct Game {
    pub command_sender: Sender<ChatCommand>,
    pub end_result: Option<EndResult>,
    pub paused: Arc<AtomicBool>,
    last_kill: Option<Vec<u8>>,
    day_night_count: DayNightCount,
}

impl Game {
    pub fn new(command_sender: Sender<ChatCommand>, paused: Arc<AtomicBool>) -> Self {
        Self {
            command_sender,
            end_result: None,
            paused,
            last_kill: None,
            day_night_count: DayNightCount {
                day_count: 0,
                night_count: 0,
                is_night: false,
            },
        }
    }

    pub async fn check_pause(&self) {
        while self.paused.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub fn check_end(&mut self) -> Option<EndResult> {
        let actors = Self::get_nondead_actors();
        let townies = actors
            .iter()
            .filter(|actor| !matches!(actor.role.alignment(), RoleAlignment::Mafia))
            .collect::<Vec<_>>();
        let mafias = actors
            .iter()
            .filter(|actor| matches!(actor.role.alignment(), RoleAlignment::Mafia))
            .collect::<Vec<_>>();
        if townies.len() == mafias.len() {
            Some(crate::game::EndResult::Mafia)
        } else if mafias.is_empty() {
            Some(crate::game::EndResult::Town)
        } else {
            None
        }
    }
}

pub struct DayNightCount {
    pub day_count: u8,
    pub night_count: u8,
    pub is_night: bool,
}

pub enum EndResult {
    Mafia,
    Town,
}
