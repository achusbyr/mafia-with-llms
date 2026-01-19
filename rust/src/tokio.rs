use godot::prelude::*;
use tokio::runtime;

#[derive(GodotClass)]
#[class(singleton, base = Object)]
pub struct AsyncRuntime {
    pub runtime: runtime::Runtime,
    base: Base<Object>,
}

#[godot_api]
impl IObject for AsyncRuntime {
    fn init(base: Base<Self::Base>) -> Self {
        let runtime = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        Self { runtime, base }
    }
}
