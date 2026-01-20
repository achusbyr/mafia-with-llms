#[derive(Clone)]
pub enum Action {
    Abstain,
    Whisper(u8, String),
    TagPlayerForComment(u8),
    ProvideID(u8),
    Talk(String),
    MultiCall(Vec<Action>),
}
