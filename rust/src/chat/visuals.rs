use std::f64;

use godot::{
    classes::{
        class_macros::private::virtuals::Os::{real, Vector3}, AnimatableBody3D, Label3D,
        Sprite3D,
    },
    meta::ToGodot,
    obj::{NewGd, Singleton, WithBaseField}
    ,
};

use crate::{actor::BaseActor, chat::Chat, load_message_scene};

impl Chat {
    pub fn focus_camera_on_actor(&mut self, actor_id: u8) {
        if let Some(target_node) = self.player_nodes.get(&actor_id) {
            let target_pos = target_node.get_global_position();
            let mut camera = self.camera.clone().unwrap();
            if let Some(mut tween) = self.base_mut().create_tween() {
                let old_rot = camera.get_global_rotation();
                camera.look_at(target_pos);
                let target_rot = camera.get_global_rotation();
                camera.set_global_rotation(old_rot);
                tween.set_trans(godot::classes::tween::TransitionType::QUART);
                tween.set_ease(godot::classes::tween::EaseType::OUT);
                tween.tween_property(
                    &camera.upcast::<godot::classes::Object>(),
                    "global_rotation",
                    &target_rot.to_variant(),
                    2.5,
                );
            }
        }
    }

    pub fn spawn_visuals(&mut self, actors: &[BaseActor]) {
        let town_center_pos = self.get_town_center().get_global_position();
        let count = actors.len() as f64;
        let radius = 2.0;

        for (index, actor) in actors.iter().enumerate() {
            let mut instance = load_message_scene().instantiate_as::<AnimatableBody3D>();
            let angle = 2.0 * f64::consts::PI / count * (index as f64);
            let offset = Vector3::FORWARD.rotated(Vector3::UP, angle as real) * radius;
            let final_pos = town_center_pos + offset;

            self.get_world()
                .call_deferred("add_child", &[instance.clone().to_variant()]);

            instance.look_at_from_position(final_pos, town_center_pos);
            instance.rotate_object_local(Vector3::UP, f64::consts::PI as real); // Spin 180
            instance.translate_object_local(Vector3 {
                x: 0.0,
                y: 0.4,
                z: 0.0,
            }); // Slightly above ground
            instance.scale_object_local(Vector3 {
                x: 0.5,
                y: 0.5,
                z: 0.5,
            });
            instance
                .get_node_as::<Label3D>("Name")
                .set_text(&actor.name);
            instance
                .get_node_as::<Label3D>("Role")
                .set_text(&actor.role.name());

            let mut body = instance.get_node_as::<godot::classes::MeshInstance3D>("Body");
            let texture = godot::classes::ResourceLoader::singleton()
                .load(&actor.model_customization.sprite_path)
                .unwrap()
                .cast::<godot::classes::Texture2D>();
            body.get_node_as::<Sprite3D>("Head/Image")
                .set_texture(&texture);
            let mut material = godot::classes::StandardMaterial3D::new_gd();
            material.set_albedo(actor.model_customization.color);
            body.set_material_override(&material);
            for child in body.get_children().iter_shared() {
                if let Ok(mut child) = child.try_cast::<godot::classes::MeshInstance3D>() {
                    let mut material = godot::classes::StandardMaterial3D::new_gd();
                    material.set_albedo(actor.model_customization.color);
                    child.set_material_override(&material);
                }
            }

            self.player_nodes.insert(actor.id, instance);
        }
    }
}
