mod actor;
pub mod context_entry;
mod data;
mod game;
mod llm;
mod main_node;
mod prompts;
mod tokio;

use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
