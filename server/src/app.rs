use crate::config::AppConfig;
use crate::engine_task::run_engine_task;
use crate::http::router;
use crate::maker_bot::{run_maker_bot, validate_maker_config};
use crate::state::AppState;
use crate::taker_bot::{run_taker_bot, validate_taker_config};
use crate::terminal_view::run_terminal_view;
use crate::types::{CommandIntent, ServerEvent};
use rock_matching_engine::Engine;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::{Semaphore, broadcast, mpsc, watch};
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(config: AppConfig) -> Result<(), Box<dyn Error>> {
    let config = config.validate()?;
    let maker_config = validate_maker_config(config.maker)?;
    let taker_config = validate_taker_config(config.taker)?;

    let tcp_listener = tokio::net::TcpListener::bind(config.bind_address).await?;

    let shutdown = CancellationToken::new();
    let (broadcast_sender, broadcast_receiver) =
        broadcast::channel::<ServerEvent>(config.event_channel_capacity);
    let (latest_event_sender, latest_event_receiver) = watch::channel(None::<ServerEvent>);
    let (command_sender, command_receiver) =
        mpsc::channel::<CommandIntent>(config.command_channel_capacity);
    let websocket_connection_semaphore = Arc::new(Semaphore::new(config.max_websocket_connections));

    let app = router(AppState {
        server_broadcast_sender: broadcast_sender.clone(),
        server_latest_event_receiver: latest_event_receiver,
        websocket_connection_semaphore,
    });

    let terminal_handle = tokio::spawn(async move { run_terminal_view(broadcast_receiver).await });

    let engine_handle = tokio::spawn(async move {
        run_engine_task(
            command_receiver,
            broadcast_sender,
            latest_event_sender,
            Engine::default(),
        )
        .await;
    });

    let maker_sender = command_sender.clone();
    let maker_shutdown = shutdown.clone();
    let maker_handle =
        tokio::spawn(
            async move { run_maker_bot(maker_sender, maker_config, maker_shutdown).await },
        );

    let taker_sender = command_sender.clone();
    let taker_shutdown = shutdown.clone();
    let taker_handle =
        tokio::spawn(
            async move { run_taker_bot(taker_sender, taker_config, taker_shutdown).await },
        );

    println!("server running; press Ctrl+C to stop");
    let server_result = axum::serve(tcp_listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown.clone()))
        .await;

    shutdown.cancel();

    match maker_handle.await {
        Ok(Ok(())) => println!("maker stopped cleanly"),
        Ok(Err(error)) => eprintln!("maker failed to send command: {error:?}"),
        Err(error) => eprintln!("maker task failed: {error:?}"),
    }

    match taker_handle.await {
        Ok(Ok(())) => println!("taker stopped cleanly"),
        Ok(Err(error)) => eprintln!("taker failed to send command: {error:?}"),
        Err(error) => eprintln!("taker task failed: {error:?}"),
    }

    drop(command_sender);

    let engine_result = engine_handle.await;
    let terminal_result = terminal_handle.await;

    server_result?;

    engine_result?;
    println!("engine task stopped cleanly");

    terminal_result?;
    println!("listener stopped cleanly");

    Ok(())
}

async fn shutdown_signal(shutdown: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }

    println!("shutdown requested");
    shutdown.cancel();
}
