use crate::types::ServerEvent;
use rock_matching_engine::Level;
use std::fmt::Write;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

pub(crate) async fn run_terminal_view(receiver: broadcast::Receiver<ServerEvent>) {
    consume_server_events(receiver, print_server_event).await;
}

async fn consume_server_events<F>(mut receiver: broadcast::Receiver<ServerEvent>, mut consume: F)
where
    F: FnMut(&ServerEvent),
{
    loop {
        match receiver.recv().await {
            Ok(server_event) => consume(&server_event),
            Err(RecvError::Lagged(messages)) => {
                eprintln!("terminal listener skipped {messages} messages");
            }
            Err(RecvError::Closed) => break,
        }
    }
}

fn print_server_event(server_event: &ServerEvent) {
    println!(
        "bids: {:?}, asks: {:?}, last_price: {:?}",
        format_levels(&server_event.snapshot.bids),
        format_levels(&server_event.snapshot.asks),
        server_event
            .last_price
            .map(|price| price.0.to_string())
            .unwrap_or_else(|| "No price".to_string())
    );
}

pub(crate) fn format_levels(levels: &[Level]) -> String {
    let mut formatted_levels = String::new();
    for (i, level) in levels.iter().enumerate() {
        if i > 0 {
            formatted_levels.push(' ');
        }
        write!(
            &mut formatted_levels,
            "{:?}x{:?}",
            level.price.0, level.quantity.0
        )
        .unwrap();
    }

    formatted_levels
}

#[cfg(test)]
mod tests {
    use super::*;
    use rock_matching_engine::{Engine, Price};

    #[tokio::test]
    async fn continues_consuming_after_lag_and_stops_when_closed() {
        let (sender, receiver) = broadcast::channel(1);
        let snapshot = Engine::default().top_levels(10);

        sender
            .send(ServerEvent {
                snapshot: snapshot.clone(),
                last_price: Some(Price(100)),
            })
            .expect("terminal receiver should remain open");
        sender
            .send(ServerEvent {
                snapshot,
                last_price: Some(Price(101)),
            })
            .expect("terminal receiver should remain open");
        drop(sender);

        let mut received_prices = Vec::new();
        consume_server_events(receiver, |event| {
            received_prices.push(event.last_price);
        })
        .await;

        assert_eq!(received_prices, vec![Some(Price(101))]);
    }
}
