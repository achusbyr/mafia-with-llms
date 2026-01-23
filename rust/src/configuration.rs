use godot::{
    classes::{Button, CheckBox, Control, IControl, Label, LineEdit, VBoxContainer},
    prelude::*,
};
use crate::load_world_scene;

pub type Config = (
    bool,
    (String, String),
    Option<u8>,
    Vec<crate::actor::BaseActor>,
);

pub static mut CONFIGURATION: Option<Config> = None;

#[derive(GodotClass)]
#[class(base = Control)]
struct Configuration {
    model_pool: Vec<ModelNameID>,
    selected_entry: Option<Gd<ActorEntry>>,
    base: Base<Control>,
}

#[godot_api]
impl IControl for Configuration {
    fn init(base: Base<Control>) -> Self {
        Self {
            model_pool: vec![ModelNameID {
                model_id: "tngtech/tng-r1t-chimera:free".to_string(),
                display_name: "DeepSeek".to_string(),
                model_customization: ModelCustomization {
                    sprite_path: "res://images/deepseek.png".to_string(),
                    color: Color::LIGHT_BLUE,
                },
            }],
            selected_entry: None,
            base,
        }
    }

    fn ready(&mut self) {
        let mut api_key = self
            .base()
            .get_node_as::<LineEdit>("Root UI/Main Controls/API Key");
        if let Some(key) = godot::classes::FileAccess::open(
            "res://API_KEY.txt",
            godot::classes::file_access::ModeFlags::READ,
        ) {
            api_key.set_text(&key.get_as_text());
        }
        let add_as_playable = self
            .base()
            .get_node_as::<CheckBox>("Root UI/List Controls/Add As Playable");
        let mut actor_list = self.obtain_actor_list();
        self.base()
            .get_node_as::<Button>("Root UI/List Controls/Add")
            .signals()
            .pressed()
            .connect_self(move |button| {
                let mut config = button.get_node_as::<Configuration>("../../..");
                let model = config.bind_mut().take_model();
                let entry = if add_as_playable.is_pressed() {
                    for existing in actor_list.get_children().iter_shared() {
                        let entry = existing.cast::<ActorEntry>();
                        if let ActorBlueprint::Real(..) =
                            entry.bind().actor_blueprint.as_ref().unwrap()
                        {
                            return;
                        }
                    }
                    construct_entry(ActorBlueprint::Real("Player".to_string()))
                } else {
                    construct_entry(ActorBlueprint::Llm(model))
                };
                actor_list.add_child(&entry);
            });
        let mut selected = self
            .base()
            .get_node_as::<Label>("Root UI/List Controls/Selected");
        let actor_list = self.obtain_actor_list();
        self.base()
            .get_node_as::<Button>("Root UI/List Controls/Remove")
            .signals()
            .pressed()
            .connect_self(move |button| {
                let mut configuration = button.get_node_as::<Configuration>("../../..");
                let removing_entry = configuration.bind().selected_entry.clone();
                if let Some(removing_entry) = removing_entry {
                    for mut existing in actor_list.get_children().iter_shared() {
                        let entry = existing.clone().cast::<ActorEntry>();
                        if entry == removing_entry {
                            existing.queue_free();
                            selected.set_text("Currently Selected: None");
                            configuration.bind_mut().selected_entry = None;
                            if let ActorBlueprint::Llm(model_name_id) =
                                entry.bind().actor_blueprint.as_ref().unwrap()
                            {
                                configuration.bind_mut().return_model(model_name_id.clone());
                            }
                            break;
                        }
                    }
                }
            });
        let api_url = self
            .base()
            .get_node_as::<LineEdit>("Root UI/Main Controls/API URL");
        let start_at_night = self
            .base()
            .get_node_as::<CheckBox>("Root UI/Main Controls/Start At Night");
        let actor_list = self.obtain_actor_list();
        self.base()
            .get_node_as::<Button>("Root UI/Main Controls/Padding/Begin")
            .signals()
            .pressed()
            .connect_self(move |config| {
                let mut tree = config.get_tree().unwrap();
                tree.change_scene_to_packed(&load_world_scene());
                let mut playable_actor: Option<u8> = None;
                let actors = {
                    let mut actors = Vec::new();
                    for (index, existing) in actor_list.get_children().iter_shared().enumerate() {
                        let entry = existing.cast::<ActorEntry>();
                        let entry = entry.bind();
                        match entry.actor_blueprint.as_ref().unwrap() {
                            ActorBlueprint::Real(name) => {
                                actors.push(crate::actor::BaseActor {
                                    name: name.clone(),
                                    id: index as u8,
                                    role: crate::data::roles::GameRole::Villager,
                                    extra_data: vec![],
                                    kind: crate::actor::ActorKind::Real,
                                    model_customization: ModelCustomization {
                                        sprite_path: "res://images/user.png".to_string(),
                                        color: Color::WHITE,
                                    },
                                });
                                playable_actor = Some(index as u8);
                            }
                            ActorBlueprint::Llm(model_name_id) => {
                                actors.push(crate::actor::BaseActor {
                                    name: model_name_id.display_name.clone(),
                                    id: index as u8,
                                    role: crate::data::roles::GameRole::Villager,
                                    extra_data: vec![],
                                    kind: crate::actor::ActorKind::Llm(
                                        crate::llm::ai_interface::AIInterface {
                                            model_id: model_name_id.model_id.clone(),
                                            owner_id: index as u8,
                                        },
                                    ),
                                    model_customization: model_name_id.model_customization.clone(),
                                });
                            }
                        }
                    }
                    actors
                };
                unsafe {
                    CONFIGURATION = Some((
                        start_at_night.is_pressed(),
                        (
                            api_key.get_text().to_string().trim().to_string(),
                            api_url.get_text().to_string(),
                        ),
                        playable_actor,
                        actors,
                    ));
                }
            });
    }
}

impl Configuration {
    fn obtain_actor_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Root UI/List BG/Scroll/Actor List")
    }

    pub fn take_model(&mut self) -> ModelNameID {
        let final_model: ModelNameID;
        if let Some(model) = self.model_pool.pop() {
            final_model = model.clone();
        } else {
            final_model = ModelNameID {
                model_id: "tngtech/tng-r1t-chimera:free".to_string(),
                display_name: "Use My ID Instead".to_string(),
                model_customization: ModelCustomization {
                    sprite_path: "res://images/openai.png".to_string(),
                    color: Color::WHITE,
                },
            };
        }
        self.model_pool
            .retain(|x| x.model_id != final_model.model_id);
        final_model
    }

    pub fn return_model(&mut self, model: ModelNameID) {
        self.model_pool.push(model);
    }
}

fn construct_entry(blueprint: ActorBlueprint) -> Gd<ActorEntry> {
    let mut entry = ActorEntry::new_alloc();
    let name = match &blueprint {
        ActorBlueprint::Llm(model_name_id) => model_name_id.display_name.clone(),
        ActorBlueprint::Real(name) => format!("{} (Playable)", name),
    };
    entry.bind_mut().actor_blueprint = Some(blueprint);
    entry.set_text(&name);
    entry.set_text_alignment(godot::global::HorizontalAlignment::LEFT);
    let button = entry.bind();
    let entry_clone = entry.clone();
    button
        .base()
        .signals()
        .pressed()
        .connect_self(move |button| {
            let mut selected = button.get_node_as::<Label>("../../../../List Controls/Selected");
            selected.set_text(&format!(
                "Currently Selected: {}",
                match entry_clone.bind().actor_blueprint.as_ref().unwrap() {
                    ActorBlueprint::Llm(model_name_id) => model_name_id.display_name.clone(),
                    ActorBlueprint::Real(name) => name.clone(),
                }
            ));
            let mut configuration = button.get_node_as::<Configuration>("../../../../..");
            configuration.bind_mut().selected_entry = Some(entry_clone.clone());
        });
    entry.clone()
}

#[derive(GodotClass)]
#[class(init, base = Button)]
struct ActorEntry {
    pub actor_blueprint: Option<ActorBlueprint>,
    base: Base<Button>,
}

pub enum ActorBlueprint {
    Real(String),
    Llm(ModelNameID),
}

impl PartialEq for ActorBlueprint {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Real(l0), Self::Real(r0)) => l0 == r0,
            (Self::Llm(l0), Self::Llm(r0)) => l0.display_name == r0.display_name,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct ModelNameID {
    pub model_id: String,
    pub display_name: String,
    pub model_customization: ModelCustomization,
}

#[derive(Clone)]
pub struct ModelCustomization {
    pub sprite_path: String,
    pub color: Color,
}
