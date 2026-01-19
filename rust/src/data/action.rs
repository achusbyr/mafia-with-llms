#[derive(Clone)]
pub enum Action {
    Abstain,
    Whisper { to: u8, message: String },
    TagPlayerForComment { id: u8 },
    ProvideID(u8),
    Talk(String),
    MultiCall(Vec<Action>),
}
