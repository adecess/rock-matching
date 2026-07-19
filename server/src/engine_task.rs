use crate::types::{CommandIntent, ServerEvent};
use rock_matching_engine::{Command, Engine, Event, Timestamp};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::sync::watch;

pub(crate) async fn run_engine_task(
    mut mpsc_receiver: Receiver<CommandIntent>,
    broadcast_sender: Sender<ServerEvent>,
    latest_event_sender: watch::Sender<Option<ServerEvent>>,
    mut engine: Engine,
) {
    let mut last_price = None;
    let mut next_timestamp = 0u64;

    while let Some(command_intent) = mpsc_receiver.recv().await {
        next_timestamp += 1;

        let command = intent_to_command(command_intent, next_timestamp);
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
                // Store the latest snapshot in the axum state
                latest_event_sender.send_replace(Some(server_event.clone()));
                let _ = broadcast_sender.send(server_event);
            }
            Err(error) => {
                eprintln!("failed to apply command: {error:?}");
            }
        }
    }
}

fn intent_to_command(command_intent: CommandIntent, next_timestamp: u64) -> Command {
    match command_intent {
        CommandIntent::SubmitOrder {
            quantity,
            side,
            order_type,
        } => Command::SubmitOrder {
            timestamp: Timestamp(next_timestamp),
            quantity,
            side,
            order_type,
        },

        CommandIntent::CancelOrder { order_id } => Command::CancelOrder {
            timestamp: Timestamp(next_timestamp),
            order_id,
        },
    }
}
