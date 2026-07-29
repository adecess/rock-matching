mod app;
mod config;
mod engine_task;
mod http;
mod maker_bot;
mod state;
mod taker_bot;
mod terminal_view;
mod types;

use crate::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run(AppConfig::default()).await
}
