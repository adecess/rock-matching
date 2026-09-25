use crate::types::ServerEvent;
use std::sync::Arc;
use tokio::sync::{Semaphore, broadcast, watch};

#[derive(Clone)]
pub struct AppState {
    pub(crate) server_broadcast_sender: broadcast::Sender<ServerEvent>,
    pub(crate) server_latest_event_receiver: watch::Receiver<Option<ServerEvent>>,
    pub(crate) websocket_connection_semaphore: Arc<Semaphore>,
    pub(crate) allowed_origins: Arc<[String]>,
}
