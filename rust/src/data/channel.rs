#[derive(Clone)]
pub enum Channel {
    Global,
    Mafia,
    ToSelf(u8),
    Raw(u8),
}
