use crate::state::AppState;
use crate::types::ServerEvent;
use axum::extract::ws::{CloseFrame, Message, WebSocket, close_code};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::header::ORIGIN;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{OwnedSemaphorePermit, TryAcquireError, watch};

const WEBSOCKET_READ_BUFFER_SIZE: usize = 4 * 1024;
const WEBSOCKET_WRITE_BUFFER_SIZE: usize = 16 * 1024;
const MAX_WEBSOCKET_MESSAGE_SIZE: usize = 16 * 1024;
const MAX_WEBSOCKET_FRAME_SIZE: usize = 16 * 1024;
const MAX_WEBSOCKET_WRITE_BUFFER_SIZE: usize = 256 * 1024;

#[derive(Debug, PartialEq)]
enum ClientMessageAction {
    Continue,
    Disconnect,
    Reject,
}

pub(crate) async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if !origin_allowed(&headers, &state.allowed_origins) {
        return (StatusCode::FORBIDDEN, "websocket origin not allowed").into_response();
    }

    let connection_permit = match acquire_connection_permit(&state) {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "websocket connection limit reached",
            )
                .into_response();
        }
    };

    let broadcast_rx = state.server_broadcast_sender.subscribe();
    let latest_event_receiver = state.server_latest_event_receiver;

    ws.read_buffer_size(WEBSOCKET_READ_BUFFER_SIZE)
        .write_buffer_size(WEBSOCKET_WRITE_BUFFER_SIZE)
        .max_message_size(MAX_WEBSOCKET_MESSAGE_SIZE)
        .max_frame_size(MAX_WEBSOCKET_FRAME_SIZE)
        .max_write_buffer_size(MAX_WEBSOCKET_WRITE_BUFFER_SIZE)
        .on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error))
        .on_upgrade(|socket| {
            handle_socket(
                socket,
                broadcast_rx,
                latest_event_receiver,
                connection_permit,
            )
        })
}

fn origin_allowed(headers: &HeaderMap, allowed_origins: &[String]) -> bool {
    let mut origins = headers.get_all(ORIGIN).iter();
    let Some(origin) = origins.next() else {
        return false;
    };

    origins.next().is_none()
        && allowed_origins
            .iter()
            .any(|allowed| allowed.as_bytes() == origin.as_bytes())
}

fn acquire_connection_permit(state: &AppState) -> Result<OwnedSemaphorePermit, TryAcquireError> {
    state
        .websocket_connection_semaphore
        .clone()
        .try_acquire_owned()
}

async fn handle_socket(
    mut socket: WebSocket,
    mut broadcast_rx: Receiver<ServerEvent>,
    latest_event_receiver: watch::Receiver<Option<ServerEvent>>,
    _connection_permit: OwnedSemaphorePermit,
) {
    // Send the latest snapshot immediately on connection
    let latest_event = latest_event_receiver.borrow().clone();
    if let Some(latest_event) = latest_event {
        let result = socket
            .send(Message::from(serde_json::to_string(&latest_event).unwrap()))
            .await;

        if let Err(error) = result {
            println!("Error sending latest event: {}", error);
            return;
        }
    }

    loop {
        tokio::select! {
            // broadcast receiver
            server_event =  broadcast_rx.recv() => {
                match server_event {
                    Ok(server_event) => {
                        let result = socket
                        .send(Message::from(serde_json::to_string(&server_event).unwrap()))
                        .await;

                        if let Err(error) = result {
                            println!("Error sending: {}", error);
                            break;
                        }

                        continue;
                    },
                    Err(RecvError::Closed) => {
                        break;
                    },
                    Err(RecvError::Lagged(messages)) => {
                        println!("receiver lagged too far behind, {} messages skipped", messages);
                    }
                }
            }
            // websocket receiver
            message = socket.recv() => {
                match message {
                    Some(Ok(message)) => match client_message_action(&message) {
                        ClientMessageAction::Continue => {}
                        ClientMessageAction::Disconnect => break,
                        ClientMessageAction::Reject => {
                            let close = Message::Close(Some(CloseFrame {
                                code: close_code::POLICY,
                                reason: "client data messages are not supported".into(),
                            }));
                            if let Err(error) = socket.send(close).await {
                                println!("Error rejecting client message: {error}");
                            }
                            break;
                        }
                    },
                    Some(Err(error)) => {
                        println!("Error receiving websocket message: {error}");
                        break;
                    }
                    None => break,
                }
            }
        }
    }
}

fn client_message_action(message: &Message) -> ClientMessageAction {
    match message {
        Message::Ping(_) | Message::Pong(_) => ClientMessageAction::Continue,
        Message::Close(_) => ClientMessageAction::Disconnect,
        Message::Text(_) | Message::Binary(_) => ClientMessageAction::Reject,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{Semaphore, broadcast, watch};

    fn state_with_connection_limit(limit: usize) -> AppState {
        let (server_broadcast_sender, _) = broadcast::channel(1);
        let (_, server_latest_event_receiver) = watch::channel(None);

        AppState {
            server_broadcast_sender,
            server_latest_event_receiver,
            websocket_connection_semaphore: Semaphore::new(limit).into(),
            allowed_origins: vec!["http://localhost:5173".to_owned()].into(),
        }
    }

    #[test]
    fn connection_permit_is_released_when_connection_ends() {
        let state = state_with_connection_limit(1);
        let permit = acquire_connection_permit(&state).expect("first connection should be allowed");

        assert!(acquire_connection_permit(&state).is_err());

        drop(permit);

        assert!(acquire_connection_permit(&state).is_ok());
    }

    #[test]
    fn accepts_control_messages() {
        assert_eq!(
            client_message_action(&Message::Ping(Vec::new().into())),
            ClientMessageAction::Continue
        );
        assert_eq!(
            client_message_action(&Message::Pong(Vec::new().into())),
            ClientMessageAction::Continue
        );
        assert_eq!(
            client_message_action(&Message::Close(None)),
            ClientMessageAction::Disconnect
        );
    }

    #[test]
    fn rejects_client_data_messages() {
        assert_eq!(
            client_message_action(&Message::Text("order".into())),
            ClientMessageAction::Reject
        );
        assert_eq!(
            client_message_action(&Message::Binary(Vec::new().into())),
            ClientMessageAction::Reject
        );
    }

    #[test]
    fn only_accepts_a_single_configured_origin() {
        let allowed = ["https://app.example.com".to_owned()];
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://app.example.com".parse().unwrap());
        assert!(origin_allowed(&headers, &allowed));

        headers.insert(ORIGIN, "https://other.example.com".parse().unwrap());
        assert!(!origin_allowed(&headers, &allowed));

        headers.remove(ORIGIN);
        assert!(!origin_allowed(&headers, &allowed));

        headers.append(ORIGIN, "https://app.example.com".parse().unwrap());
        headers.append(ORIGIN, "https://app.example.com".parse().unwrap());
        assert!(!origin_allowed(&headers, &allowed));
    }
}
