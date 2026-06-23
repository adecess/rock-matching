use rock_matching_engine::{BookSnapshot, Price};

#[derive(Debug, Clone)]
pub(crate) struct ServerEvent {
    pub(crate) snapshot: BookSnapshot,
    pub(crate) last_price: Option<Price>,
}
