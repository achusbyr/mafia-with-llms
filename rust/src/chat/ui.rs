use godot::{classes::Window, obj::Gd};
use godot::{
    classes::{Button, Control, Label, MeshInstance3D, Node3D, TextEdit, VBoxContainer},
    obj::WithBaseField,
};

use crate::chat::Chat;
use crate::data::context_entry::SayerType;
use crate::load_message_scene;

impl Chat {
    pub fn setup_menu(&self) {
        self.base()
            .get_node_as::<Window>("Messages Window")
            .signals()
            .close_requested()
            .connect_self(|window| {
                window.hide();
            });
        let menu_button = self.get_menu_button();
        let mut menu = self.get_menu();
        let close = menu.get_node_as::<Button>("Background/Margin/Container/Close");
        let open_messages = menu.get_node_as::<Button>("Background/Margin/Container/Open Messages");
        let developer_window =
            menu.get_node_as::<Button>("Background/Margin/Container/Developer Window");
        open_messages.signals().pressed().connect_self(|button| {
            let chat = button.get_node_as::<Chat>("../../../../..");
            chat.get_node_as::<Window>("Messages Window").show();
        });
        developer_window.signals().pressed().connect_self(|button| {
            let chat = button.get_node_as::<Chat>("../../../../..");
            chat.get_node_as::<Window>("Developer Window").show();
        });
        close.signals().pressed().connect(move || {
            menu.hide();
        });
        let mut menu = self.get_menu();
        menu_button.signals().pressed().connect(move || {
            menu.show();
        });
    }

    #[cfg(feature = "development")]
    pub fn setup_developer_window(&self) {
        self.get_menu()
            .get_node_as::<Button>("Background/Margin/Container/Developer Window")
            .show();
        let mut developer_window = self.get_development_window();
        developer_window
            .signals()
            .close_requested()
            .connect_self(|window| {
                window.hide();
            });
        developer_window.set_visible(true);
        let developer = developer_window.get_node_as::<Control>("Developer");
        let id_select = developer
            .get_node_as::<godot::classes::OptionButton>("Root UI/Control Panel/ID Select");
        id_select.signals().pressed().connect_self(|button| {
            button.clear();
            for actor in crate::game::Game::get_actors() {
                button.add_item(&format!("{} (ID {})", actor.name, actor.id));
            }
        });
        developer
            .get_node_as::<Button>("Root UI/Control Panel/Build")
            .signals()
            .pressed()
            .connect(move || {
                let context = crate::game::Game::get_context()
                    .clone()
                    .into_iter()
                    .filter(|entry| {
                        entry.available_for_actor(
                            crate::game::Game::get_actor_from_id(id_select.get_selected_id() as u8)
                                .unwrap(),
                            developer
                                .get_node_as::<godot::classes::CheckBox>(
                                    "Root UI/Control Panel/Include Raw",
                                )
                                .is_pressed(),
                        )
                    })
                    .collect::<Vec<_>>();
                let mut messages =
                    developer.get_node_as::<VBoxContainer>("Root UI/Scroll/Messages");
                for mut existing in messages.get_children().iter_shared() {
                    existing.queue_free();
                }
                for entry in context {
                    let message = load_message_scene().instantiate_as::<Control>();
                    match entry.sayer_type {
                        SayerType::Actor(id) => {
                            let actor = crate::game::Game::get_actors()
                                .iter()
                                .find(|actor| actor.id == id)
                                .unwrap();
                            message
                                .get_node_as::<Label>("Container/Background/Sayer")
                                .set_text(&format!("{} (ID {})", actor.name, actor.id));
                        }
                        SayerType::System => {
                            message
                                .get_node_as::<Label>("Container/Background/Sayer")
                                .set_text("System");
                        }
                    }
                    message
                        .get_node_as::<godot::classes::RichTextLabel>("Container/Content")
                        .set_text(&entry.content);
                    messages.add_child(&message);
                }
            });
    }

    #[cfg(feature = "development")]
    pub fn get_development_window(&self) -> Gd<Window> {
        self.base().get_node_as::<Window>("Developer Window")
    }

    pub fn get_message_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Messages Window/Msg BG/Scroll/Messages")
    }

    pub fn get_actor_list(&self) -> Gd<VBoxContainer> {
        self.base()
            .get_node_as::<VBoxContainer>("Actors BG/Scroll/Actors")
    }

    pub fn get_input_box(&self) -> Gd<TextEdit> {
        self.base()
            .get_node_as::<TextEdit>("Controls BG/Controls/Input")
    }

    pub fn get_send_button(&self) -> Gd<Button> {
        self.base()
            .get_node_as::<Button>("Controls BG/Controls/Send")
    }

    pub fn get_menu_button(&self) -> Gd<Button> {
        self.base().get_node_as::<Button>("MenuButton")
    }

    pub fn get_menu(&self) -> Gd<Control> {
        self.base().get_node_as::<Control>("Menu")
    }

    pub fn get_current_text(&self) -> Gd<Label> {
        self.base().get_node_as::<Label>("Text BG/Current Text")
    }

    pub fn get_town_center(&self) -> Gd<MeshInstance3D> {
        self.get_world()
            .get_node_as::<MeshInstance3D>("Town Center")
    }

    pub fn get_world(&self) -> Gd<Node3D> {
        self.base().get_node_as::<Node3D>("../..")
    }
}
