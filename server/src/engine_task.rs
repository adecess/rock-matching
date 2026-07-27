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

#[cfg(test)]
mod tests {
    use super::*;
    use rock_matching_engine::{OrderType, Price, Qty, Side};
    use std::time::Duration;
    use tokio::sync::{broadcast, mpsc, watch};
    use tokio::time::timeout;

    #[tokio::test]
    async fn publishes_snapshots_and_tracks_the_last_trade_price() {
        let (command_sender, command_receiver) = mpsc::channel(2);
        let (broadcast_sender, mut broadcast_receiver) = broadcast::channel(2);
        let (latest_event_sender, latest_event_receiver) = watch::channel(None);

        let task = tokio::spawn(run_engine_task(
            command_receiver,
            broadcast_sender,
            latest_event_sender,
            Engine::default(),
        ));

        command_sender
            .send(CommandIntent::SubmitOrder {
                quantity: Qty(5),
                side: Side::Sell,
                order_type: OrderType::Limit(Price(100)),
            })
            .await
            .expect("engine task should accept a limit order");

        let event = timeout(Duration::from_secs(1), broadcast_receiver.recv())
            .await
            .expect("engine task should publish an event")
            .expect("broadcast channel should remain open");
        assert_eq!(event.last_price, None);
        assert!(event.snapshot.bids.is_empty());
        assert_eq!(event.snapshot.asks.len(), 1);
        assert_eq!(event.snapshot.asks[0].price, Price(100));
        assert_eq!(event.snapshot.asks[0].quantity, Qty(5));

        command_sender
            .send(CommandIntent::SubmitOrder {
                quantity: Qty(2),
                side: Side::Buy,
                order_type: OrderType::Market,
            })
            .await
            .expect("engine task should accept a market order");

        let event = timeout(Duration::from_secs(1), broadcast_receiver.recv())
            .await
            .expect("engine task should publish an event")
            .expect("broadcast channel should remain open");
        assert_eq!(event.last_price, Some(Price(100)));
        assert!(event.snapshot.bids.is_empty());
        assert_eq!(event.snapshot.asks.len(), 1);
        assert_eq!(event.snapshot.asks[0].price, Price(100));
        assert_eq!(event.snapshot.asks[0].quantity, Qty(3));

        // check latest event available
        {
            let latest_event = latest_event_receiver.borrow();
            let latest_event = latest_event
                .as_ref()
                .expect("watch channel should contain the latest event");
            assert_eq!(latest_event.last_price, Some(Price(100)));
            assert_eq!(latest_event.snapshot, event.snapshot);
        }

        drop(command_sender);
        timeout(Duration::from_secs(1), task)
            .await
            .expect("engine task should stop when the command channel closes")
            .expect("engine task should not panic");
    }
}
