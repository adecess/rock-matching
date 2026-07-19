use crate::state::AppState;
use crate::types::ServerEvent;
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;

pub(crate) async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    let broadcast_rx = state.broadcast_sender.subscribe();

    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error))
        .on_upgrade(|socket| handle_socket(socket, broadcast_rx))
}

async fn handle_socket(mut socket: WebSocket, mut broadcast_rx: Receiver<ServerEvent>) {
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
                        println!("receiver lagged too far behind, {} messages skippped", messages);
                    }
                }
            }
            // websocket receiver
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}
