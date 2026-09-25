mod websocket;

use websocket::websocket_handler;

use crate::state::AppState;
use axum::{Router, routing::get};

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(websocket_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ServerEvent;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tokio::sync::{Semaphore, broadcast, watch};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok_without_engine_activity() {
        let (server_broadcast_sender, _) = broadcast::channel(1);
        let (_, server_latest_event_receiver) = watch::channel(None::<ServerEvent>);
        let state = AppState {
            server_broadcast_sender,
            server_latest_event_receiver,
            websocket_connection_semaphore: Semaphore::new(1).into(),
            allowed_origins: vec!["http://localhost:5173".to_owned()].into(),
        };

        let response = router(state)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), b"ok");
    }
}
