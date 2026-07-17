use crate::types::ServerEvent;
use tokio::sync::broadcast::Sender;

#[derive(Clone)]
pub struct AppState {
    pub(crate) broadcast_sender: Sender<ServerEvent>,
}

impl AppState {
    pub fn new(broadcast_sender: Sender<ServerEvent>) -> Self {
        Self { broadcast_sender }
    }
}
