mod engine_task;
mod types;

use crate::engine_task::run_engine_task;
use crate::types::ServerEvent;
use rock_matching_engine::Command::SubmitOrder;
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{Command, Engine, Price, Qty, Side, Timestamp};
use tokio::sync::broadcast;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let engine = Engine::default();

    let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<ServerEvent>(16);
    let listener_handle = tokio::spawn(async move {
        while let Ok(server_event) = broadcast_rx.recv().await {
            println!(
                "bids: {:?}, asks: {:?}, last_price: {:?}",
                server_event.snapshot.bids, server_event.snapshot.asks, server_event.last_price
            );
        }
    });

    let (tx, rx) = mpsc::channel::<Command>(100);
    let engine_handle = tokio::spawn(async move {
        run_engine_task(rx, broadcast_tx, engine).await;
    });

    tx.send(SubmitOrder {
        timestamp: Timestamp(1),
        quantity: Qty(4),
        side: Side::Buy,
        order_type: Limit(Price(102)),
    })
    .await
    .unwrap();

    tx.send(SubmitOrder {
        timestamp: Timestamp(2),
        quantity: Qty(1),
        side: Side::Sell,
        order_type: Limit(Price(101)),
    })
    .await
    .unwrap();

    tx.send(SubmitOrder {
        timestamp: Timestamp(3),
        quantity: Qty(1),
        side: Side::Buy,
        order_type: Limit(Price(100)),
    })
    .await
    .unwrap();

    drop(tx);
    engine_handle.await.unwrap();
    listener_handle.await.unwrap();
}
