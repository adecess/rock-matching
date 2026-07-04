mod engine_task;
mod maker_bot;
mod taker_bot;
mod terminal_view;
mod types;

use crate::engine_task::run_engine_task;
use crate::maker_bot::{run_maker_bot, validate_maker_config};
use crate::taker_bot::{run_taker_bot, validate_taker_config};
use crate::terminal_view::format_levels;
use crate::types::{CommandIntent, MakerBotConfig, ServerEvent, TakerBotConfig};
use axum::{Router, routing::get};
use rock_matching_engine::{Engine, Price, Qty};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shutdown = CancellationToken::new();
    let engine = Engine::default();

    let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<ServerEvent>(16);
    let listener_handle = tokio::spawn(async move {
        while let Ok(server_event) = broadcast_rx.recv().await {
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
    });

    let (tx, rx) = mpsc::channel::<CommandIntent>(100);
    let engine_handle = tokio::spawn(async move {
        run_engine_task(rx, broadcast_tx, engine).await;
    });

    let maker_tx = tx.clone();
    let maker_config = validate_maker_config(MakerBotConfig {
        reference_price: Price(100),
        half_spread: Price(1),
        quantity: Qty(1),
        delay_ms: 100,
    })?;
    let maker_shutdown = shutdown.clone();
    let maker_handle =
        tokio::spawn(async move { run_maker_bot(maker_tx, maker_config, maker_shutdown).await });

    let taker_tx = tx.clone();
    let taker_config = validate_taker_config(TakerBotConfig {
        quantity: Qty(1),
        delay_ms: 1000,
    })?;
    let taker_shutdown = shutdown.clone();
    let taker_handle =
        tokio::spawn(async move { run_taker_bot(taker_tx, taker_config, taker_shutdown).await });

    let app = Router::new().route("/health", get(|| async { "Server is up on port 3000." }));
    let tcp_listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    let server_result = axum::serve(tcp_listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown.clone()))
        .await;

    println!("server running; press Ctrl+C to stop");

    shutdown.cancel();

    server_result?;

    let maker_result = maker_handle.await;
    match maker_result {
        Ok(Ok(())) => {
            println!("maker stopped cleanly");
        }
        Ok(Err(error)) => {
            eprintln!("maker failed to send command: {error:?}");
        }
        Err(error) => {
            eprintln!("maker task failed: {error:?}");
        }
    }

    let taker_result = taker_handle.await;
    match taker_result {
        Ok(Ok(())) => {
            println!("taker stopped cleanly");
        }
        Ok(Err(error)) => {
            eprintln!("taker failed to send command: {error:?}");
        }
        Err(error) => {
            eprintln!("taker task failed: {error:?}");
        }
    }

    drop(tx);

    engine_handle.await?;
    println!("engine task stopped cleanly");

    listener_handle.await?;
    println!("listener stopped cleanly");

    Ok(())
}

async fn shutdown_signal(shutdown: CancellationToken) {
    let _ = tokio::signal::ctrl_c().await;
    println!("shutdown requested");
    shutdown.cancel();
}
