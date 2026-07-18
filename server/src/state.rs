use crate::types::ServerEvent;
use tokio::sync::broadcast::Sender;

#[derive(Clone)]
pub struct AppState {
    pub(crate) broadcast_sender: Sender<ServerEvent>,
}
