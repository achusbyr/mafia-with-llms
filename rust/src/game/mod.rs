mod access;
mod discussion;
mod init;
mod iterate;
mod voting;

use crate::actor::BaseActor;
use crate::chat::ChatCommand;
use crate::data::context_entry::ContextEntry;
use crate::data::roles::RoleAlignment;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

const EXTRA_MESSAGES: u8 = 7;

static mut ACTORS: Vec<BaseActor> = Vec::new();
static mut CONTEXT: Vec<ContextEntry> = Vec::new();

pub struct Game {
    pub command_sender: Sender<ChatCommand>,
    pub end_result: Option<EndResult>,
    pub paused: Arc<AtomicBool>,
    playable_actor: Option<u8>,
    last_kill: Option<Vec<u8>>,
    day_night_count: DayNightCount,
}

impl Game {
    pub fn new(command_sender: Sender<ChatCommand>, playable_actor: Option<u8>) -> Self {
        Self {
            command_sender,
            end_result: None,
            paused: Arc::from(AtomicBool::new(false)),
            playable_actor,
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
            Some(EndResult::Mafia)
        } else if mafias.is_empty() {
            Some(EndResult::Town)
        } else {
            None
        }
    }

    pub fn send_on_behalf_of_chat(&self, command: ChatCommand) {
        self.command_sender.send(command).unwrap();
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
