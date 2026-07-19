use crate::types::ServerEvent;
use tokio::sync::broadcast;
use tokio::sync::watch;

#[derive(Clone)]
pub struct AppState {
    pub(crate) server_broadcast_sender: broadcast::Sender<ServerEvent>,
    pub(crate) server_latest_event_sender: watch::Sender<Option<ServerEvent>>,
}
