use crate::types::CommandIntent::SubmitOrder;
use crate::types::{CommandIntent, TakerBotConfig};
use rock_matching_engine::OrderType::Market;
use rock_matching_engine::Side;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::sleep;

pub(crate) fn validate_taker_config(
    config: TakerBotConfig,
) -> Result<TakerBotConfig, &'static str> {
    if config.quantity.0 == 0 {
        return Err("taker quantity must be greater than zero");
    }
    if config.delay_ms == 0 {
        return Err("taker delay_ms must be greater than zero");
    }

    Ok(TakerBotConfig {
        quantity: config.quantity,
        delay_ms: config.delay_ms,
    })
}

pub(crate) async fn run_taker_bot(
    sender: Sender<CommandIntent>,
    config: TakerBotConfig,
) -> Result<(), SendError<CommandIntent>> {
    let mut next_side = Side::Buy;

    loop {
        sleep(Duration::from_millis(config.delay_ms)).await;

        sender
            .send(SubmitOrder {
                quantity: config.quantity,
                side: next_side,
                order_type: Market,
            })
            .await?;

        next_side = match next_side {
            Side::Sell => Side::Buy,
            Side::Buy => Side::Sell,
        };
    }
}
