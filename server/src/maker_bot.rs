use crate::types::CommandIntent::SubmitOrder;
use crate::types::{CommandIntent, MakerBotConfig};
use rand::random_range;
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{Price, Qty, Side};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

pub(crate) fn validate_maker_config(
    config: MakerBotConfig,
) -> Result<MakerBotConfig, &'static str> {
    if config.max_bid_distance.0 == 0 {
        return Err("maker max_bid_distance must be greater than zero");
    }
    if config.max_ask_distance.0 == 0 {
        return Err("maker max_ask_distance must be greater than zero");
    }
    if config.min_quantity.0 == 0 {
        return Err("maker min_quantity must be greater than zero");
    }
    if config.max_quantity.0 < config.min_quantity.0 {
        return Err("maker max_quantity must be greater than min_quantity");
    }
    if config.delay_ms == 0 {
        return Err("maker delay_ms must be greater than zero");
    }

    config
        .reference_price
        .0
        .checked_sub(config.max_bid_distance.0)
        .ok_or("maker bid price would underflow")?;
    config
        .reference_price
        .0
        .checked_add(config.max_ask_distance.0)
        .ok_or("maker ask price would overflow")?;

    Ok(MakerBotConfig {
        reference_price: config.reference_price,
        max_bid_distance: config.max_bid_distance,
        max_ask_distance: config.max_ask_distance,
        max_quantity: config.max_quantity,
        min_quantity: config.min_quantity,
        delay_ms: config.delay_ms,
    })
}

pub(crate) async fn run_maker_bot(
    sender: Sender<CommandIntent>,
    config: MakerBotConfig,
    shutdown: CancellationToken,
) -> Result<(), SendError<CommandIntent>> {
    loop {
        let bid_distance = random_range(1..=config.max_bid_distance.0);
        let ask_distance = random_range(1..=config.max_ask_distance.0);

        let bid_price = Price(config.reference_price.0 - bid_distance);
        let ask_price = Price(config.reference_price.0 + ask_distance);
        let quantity = Qty(random_range(config.min_quantity.0..=config.max_quantity.0));

        sender
            .send(SubmitOrder {
                quantity,
                side: Side::Buy,
                order_type: Limit(bid_price),
            })
            .await?;

        sender
            .send(SubmitOrder {
                quantity,
                side: Side::Sell,
                order_type: Limit(ask_price),
            })
            .await?;

        tokio::select! {
            _ = sleep(Duration::from_millis(config.delay_ms)) => {
                continue
            }
            _ = shutdown.cancelled() => {
                break
            }

        }
    }

    Ok(())
}
