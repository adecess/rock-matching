use crate::types::ServerEvent;
use rock_matching_engine::{Command, Engine, Event};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;

pub(crate) async fn run_engine_task(
    mut mpsc_receiver: Receiver<Command>,
    broadcast_sender: Sender<ServerEvent>,
    mut engine: Engine,
) {
    let mut last_price = None;

    while let Some(command) = mpsc_receiver.recv().await {
        match engine.apply(command) {
            Ok(events) => {
                for event in events {
                    if let Event::OrderTraded { price, .. } = event {
                        last_price = Some(price);
                    }
                }

                let server_event = ServerEvent {
                    snapshot: engine.top_levels(10),
                    last_price,
                };
                let _ = broadcast_sender.send(server_event);
            }
            Err(error) => {
                eprintln!("failed to apply command: {error:?}");
            }
        }
    }
}
