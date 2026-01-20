mod access;
mod discussion;
mod init;
mod iterate;
mod voting;

use crate::actor::BaseActor;
use crate::chat::ChatCommand;
use crate::context_entry::ContextEntry;
use std::sync::mpsc::Sender;

const ACTOR_COUNT: u8 = 10;
const EXTRA_MESSAGES: u8 = 5;

static mut ACTORS: Vec<BaseActor> = Vec::new();
static mut CONTEXT: Vec<ContextEntry> = Vec::new();

pub struct Game {
    pub command_sender: Sender<ChatCommand>,
    pub end_result: Option<EndResult>,
    last_kill: Option<Vec<u8>>,
    day_night_count: DayNightCount,
}

impl Game {
    pub fn new(command_sender: Sender<ChatCommand>) -> Self {
        Self {
            command_sender,
            end_result: None,
            last_kill: None,
            day_night_count: DayNightCount {
                day_count: 0,
                night_count: 0,
                is_night: false,
            },
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
