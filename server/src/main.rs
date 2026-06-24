mod engine_task;
mod maker_bot;
mod types;

use crate::engine_task::run_engine_task;
use crate::maker_bot::run_maker_bot;
use crate::types::{CommandIntent, ServerEvent};
use rock_matching_engine::Engine;
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

    let (tx, rx) = mpsc::channel::<CommandIntent>(100);
    let engine_handle = tokio::spawn(async move {
        run_engine_task(rx, broadcast_tx, engine).await;
    });

    let maker_tx = tx.clone();
    let maker_handle = tokio::spawn(async move { run_maker_bot(maker_tx).await });

    drop(tx);

    if let Err(error) = maker_handle.await.unwrap() {
        eprintln!("maker bot failed: {error:?}");
    }
    engine_handle.await.unwrap();
    listener_handle.await.unwrap();
}
