use rand::seq::IndexedRandom;
use rand::seq::IteratorRandom;
use std::sync::LazyLock;

static mut MODEL_POOL: LazyLock<Vec<ModelNameID>> = LazyLock::new(|| {
    vec![ModelNameID {
        model_id: "tngtech/tng-r1t-chimera:free".to_string(),
        display_name: "DeepSeek".to_string(),
    }]
});

pub fn take_random_model() -> ModelNameID {
    #[allow(static_mut_refs)]
    let pool = unsafe { &mut MODEL_POOL };
    if pool.is_empty() {
        let names = serde_json::from_str::<Vec<&str>>(include_str!("names.json")).unwrap();
        let name = names.choose(&mut rand::rng()).unwrap();
        return ModelNameID {
            model_id: "tngtech/tng-r1t-chimera:free".to_string(),
            display_name: name.to_string(),
        };
    }
    let model = pool.iter().choose(&mut rand::rng()).unwrap().clone();
    pool.retain(|x| x.model_id != model.model_id);
    model
}

#[derive(Clone)]
pub struct ModelNameID {
    pub model_id: String,
    pub display_name: String,
}
