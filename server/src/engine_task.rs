use crate::types::{CommandIntent, ServerEvent, SubmissionReply};
use rock_matching_engine::{ApplyError, Command, Engine, Event, Timestamp};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::sync::watch;
use tracing::{debug, error};

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

        let (command, reply) = intent_to_command(command_intent, next_timestamp);
        match engine.apply(command) {
            Ok(events) => {
                if let Some(reply) = reply {
                    let resting_order_id = events.iter().find_map(|event| match event {
                        Event::OrderAddedToBook(order_id, ..) => Some(*order_id),
                        _ => None,
                    });
                    let _ = reply.send(Ok(resting_order_id));
                }
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
            Err(ApplyError::OrderNotFound(order_id)) => {
                // A taker may have fully filled a quote before its scheduled cancellation.
                debug!(
                    order_id = order_id.0,
                    "order already absent at cancellation"
                );
            }
            Err(error) => {
                error!(%error, "failed to apply command");
                if let Some(reply) = reply {
                    let _ = reply.send(Err(error));
                }
            }
        }
    }
}

fn intent_to_command(
    command_intent: CommandIntent,
    next_timestamp: u64,
) -> (Command, Option<SubmissionReply>) {
    match command_intent {
        CommandIntent::SubmitOrder {
            quantity,
            side,
            order_type,
            reply,
        } => (
            Command::SubmitOrder {
                timestamp: Timestamp(next_timestamp),
                quantity,
                side,
                order_type,
            },
            reply,
        ),

        CommandIntent::CancelOrder { order_id } => (
            Command::CancelOrder {
                timestamp: Timestamp(next_timestamp),
                order_id,
            },
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rock_matching_engine::{ApplyError, OrderId, OrderType, Price, Qty, Side};
    use std::time::Duration;
    use tokio::sync::{broadcast, mpsc, oneshot, watch};
    use tokio::time::timeout;

    #[tokio::test]
    async fn replies_with_resting_order_ids_or_submission_errors() {
        let (sender, receiver) = mpsc::channel(2);
        let (events, _) = broadcast::channel(2);
        let (latest, _) = watch::channel(None);
        let task = tokio::spawn(run_engine_task(receiver, events, latest, Engine::default()));

        let cases = [
            (
                Qty(5),
                Side::Sell,
                OrderType::Limit(Price(100)),
                Ok(Some(OrderId(0))),
            ),
            (Qty(2), Side::Buy, OrderType::Market, Ok(None)),
            // The remaining three ask units trade; only the two-unit bid rests.
            (
                Qty(5),
                Side::Buy,
                OrderType::Limit(Price(100)),
                Ok(Some(OrderId(2))),
            ),
            (
                Qty(0),
                Side::Sell,
                OrderType::Limit(Price(101)),
                Err(ApplyError::InvalidQuantity(Qty(0))),
            ),
        ];
        for (quantity, side, order_type, expected) in cases {
            let (reply, response) = oneshot::channel();
            sender
                .send(CommandIntent::SubmitOrder {
                    quantity,
                    side,
                    order_type,
                    reply: Some(reply),
                })
                .await
                .expect("command channel should remain open");
            assert_eq!(
                timeout(Duration::from_secs(1), response)
                    .await
                    .expect("submission should receive a reply")
                    .expect("reply channel should remain open"),
                expected,
            );
        }

        // Losing a reply receiver must not stop the engine task.
        let (reply, response) = oneshot::channel();
        drop(response);
        sender
            .send(CommandIntent::SubmitOrder {
                quantity: Qty(1),
                side: Side::Sell,
                order_type: OrderType::Limit(Price(101)),
                reply: Some(reply),
            })
            .await
            .expect("command channel should remain open");
        drop(sender);
        timeout(Duration::from_secs(1), task)
            .await
            .expect("engine task should drain commands and stop")
            .expect("engine task should not panic");
    }

    #[tokio::test]
    async fn publishes_snapshots_and_continues_after_stale_cancellations() {
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
                reply: None,
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
                reply: None,
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

        // Cancel the partially filled ask, then try cancelling that absent ID again.
        command_sender
            .send(CommandIntent::CancelOrder {
                order_id: OrderId(0),
            })
            .await
            .expect("engine task should accept cancellation");
        let event = timeout(Duration::from_secs(1), broadcast_receiver.recv())
            .await
            .expect("cancellation should publish a snapshot")
            .expect("broadcast channel should remain open");
        assert!(event.snapshot.asks.is_empty());
        assert_eq!(event.last_price, Some(Price(100)));

        command_sender
            .send(CommandIntent::CancelOrder {
                order_id: OrderId(0),
            })
            .await
            .expect("engine task should accept stale cancellation");
        let (reply, response) = oneshot::channel();
        command_sender
            .send(CommandIntent::SubmitOrder {
                quantity: Qty(1),
                side: Side::Buy,
                order_type: OrderType::Limit(Price(99)),
                reply: Some(reply),
            })
            .await
            .expect("engine task should accept a replacement");
        assert_eq!(
            timeout(Duration::from_secs(1), response)
                .await
                .expect("replacement should receive a reply")
                .expect("reply channel should remain open"),
            Ok(Some(OrderId(2))),
        );
        let event = timeout(Duration::from_secs(1), broadcast_receiver.recv())
            .await
            .expect("replacement should publish a snapshot")
            .expect("broadcast channel should remain open");
        assert!(event.snapshot.asks.is_empty());
        assert_eq!(event.snapshot.bids[0].quantity, Qty(1));

        drop(command_sender);
        timeout(Duration::from_secs(1), task)
            .await
            .expect("engine task should stop when the command channel closes")
            .expect("engine task should not panic");
    }
}
