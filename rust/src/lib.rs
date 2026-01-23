mod actor;
mod chat;
mod configuration;
mod data;
mod game;
mod llm;
mod prompts;
mod tokio;

use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

pub fn load_world_scene() -> Gd<PackedScene> {
    load::<PackedScene>("res://scenes/world.tscn")
}

pub fn load_message_scene() -> Gd<PackedScene> {
    load::<PackedScene>("res://scenes/message.tscn")
}

pub fn load_model_scene() -> Gd<PackedScene> {
    load::<PackedScene>("res://models/model.tscn")
}