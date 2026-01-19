use crate::actor::BaseActor;
use crate::context_entry::ContextEntry;
use crate::game::{ACTORS, CONTEXT, Game};

#[allow(static_mut_refs)]
impl Game {
    pub fn get_actors() -> &'static Vec<BaseActor> {
        unsafe { &ACTORS }
    }

    pub fn get_actors_mut() -> &'static mut Vec<BaseActor> {
        unsafe { &mut ACTORS }
    }

    pub fn get_context() -> &'static Vec<ContextEntry> {
        unsafe { &CONTEXT }
    }

    pub fn get_context_mut() -> &'static mut Vec<ContextEntry> {
        unsafe { &mut CONTEXT }
    }

    pub fn add_to_context(&mut self, entry: ContextEntry) {
        Self::get_context_mut().push(entry);
        self.refresh_context_for_actor(0);
    }

    pub fn get_nondead_actors() -> Vec<&'static BaseActor> {
        Self::get_actors()
            .iter()
            .filter(|actor| !actor.dead)
            .collect()
    }

    pub fn get_actor_from_id(id: u8) -> Option<&'static BaseActor> {
        Self::get_actors().iter().find(|actor| actor.id == id)
    }
}
