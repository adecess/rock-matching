mod websocket;

use websocket::websocket_handler;

use crate::state::AppState;
use axum::{Router, routing::get};

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "Server is up on port 3000." }))
        .route("/ws", get(websocket_handler))
        .with_state(state)
}
