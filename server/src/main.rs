mod engine_task;
mod maker_bot;
mod taker_bot;
mod types;

use crate::engine_task::run_engine_task;
use crate::maker_bot::{run_maker_bot, validate_maker_config};
use crate::taker_bot::{run_taker_bot, validate_taker_config};
use crate::types::{CommandIntent, MakerBotConfig, ServerEvent, TakerBotConfig};
use rock_matching_engine::{Engine, Price, Qty};
use tokio::sync::broadcast;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::default();

    let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<ServerEvent>(16);
    tokio::spawn(async move {
        while let Ok(server_event) = broadcast_rx.recv().await {
            println!(
                "bids: {:?}, asks: {:?}, last_price: {:?}",
                server_event.snapshot.bids, server_event.snapshot.asks, server_event.last_price
            );
        }
    });

    let (tx, rx) = mpsc::channel::<CommandIntent>(100);
    tokio::spawn(async move {
        run_engine_task(rx, broadcast_tx, engine).await;
    });

    let maker_tx = tx.clone();
    let maker_config = validate_maker_config(MakerBotConfig {
        reference_price: Price(100),
        half_spread: Price(1),
        quantity: Qty(1),
        delay_ms: 100,
    })?;
    tokio::spawn(async move { run_maker_bot(maker_tx, maker_config).await });

    let taker_tx = tx.clone();
    let taker_config = validate_taker_config(TakerBotConfig {
        quantity: Qty(1),
        delay_ms: 1000,
    })?;
    tokio::spawn(async move { run_taker_bot(taker_tx, taker_config).await });

    println!("server running; press Ctrl+C to stop");

    tokio::signal::ctrl_c().await?;

    println!("shutdown requested");

    Ok(())
}
