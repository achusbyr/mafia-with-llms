use crate::data::channel::Channel;

#[derive(Clone)]
pub enum ExtraData {
    Dead,
    ProtectedByDoctor,
    WhisperMetadata { from: u8, to: u8 },
    SaidInChannel(Channel),
}
