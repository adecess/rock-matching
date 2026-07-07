use crate::types::CommandIntent::SubmitOrder;
use crate::types::{CommandIntent, TakerBotConfig};
use rand::random_range;
use rock_matching_engine::OrderType::Market;
use rock_matching_engine::{Qty, Side};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub(crate) fn validate_taker_config(
    config: TakerBotConfig,
) -> Result<TakerBotConfig, &'static str> {
    if config.min_quantity.0 == 0 {
        return Err("taker min_quantity must be greater than zero");
    }
    if config.max_quantity.0 < config.min_quantity.0 {
        return Err("taker max_quantity must be greater than min_quantity");
    }
    if config.delay_ms == 0 {
        return Err("taker delay_ms must be greater than zero");
    }

    Ok(TakerBotConfig {
        max_quantity: config.max_quantity,
        min_quantity: config.min_quantity,
        delay_ms: config.delay_ms,
        startup_delay_ms: config.startup_delay_ms,
    })
}

pub(crate) async fn run_taker_bot(
    sender: Sender<CommandIntent>,
    config: TakerBotConfig,
    shutdown: CancellationToken,
) -> Result<(), SendError<CommandIntent>> {
    let mut next_side = Side::Buy;

    tokio::select! {
        _ =  sleep(Duration::from_millis(config.startup_delay_ms)) => {
        }
         _ = shutdown.cancelled() => {
            return Ok(())
        }
    }

    loop {
        let quantity = Qty(random_range(config.min_quantity.0..=config.max_quantity.0));

        tokio::select! {
            _ =  sleep(Duration::from_millis(config.startup_delay_ms)) => {
                continue
            }
            _ = sleep(Duration::from_millis(config.delay_ms)) => {
                sender
                .send(SubmitOrder {
                    quantity,
                    side: next_side,
                    order_type: Market,
                })
                .await?;

                next_side = match next_side {
                    Side::Sell => Side::Buy,
                    Side::Buy => Side::Sell,
                };
            }
            _ = shutdown.cancelled() => {
                break
            }
        }
    }

    Ok(())
}
