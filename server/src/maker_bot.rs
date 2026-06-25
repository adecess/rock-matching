use crate::types::CommandIntent::SubmitOrder;
use crate::types::{CommandIntent, MakerBotConfig, MakerBotRuntimeConfig};
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{Price, Side};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::{Duration, sleep};

pub(crate) fn validate_maker_config(
    config: MakerBotConfig,
) -> Result<MakerBotRuntimeConfig, &'static str> {
    if config.half_spread.0 == 0 {
        return Err("maker half_spread must be greater than zero");
    }
    if config.quantity.0 == 0 {
        return Err("maker quantity must be greater than zero");
    }
    if config.delay_ms == 0 {
        return Err("maker delay_ms must be greater than zero");
    }

    let bid = config
        .reference_price
        .0
        .checked_sub(config.half_spread.0)
        .ok_or("maker bid price would underflow")?;
    let ask = config
        .reference_price
        .0
        .checked_add(config.half_spread.0)
        .ok_or("maker bid price would overflow")?;

    Ok(MakerBotRuntimeConfig {
        bid_price: Price(bid),
        ask_price: Price(ask),
        quantity: config.quantity,
        delay_ms: config.delay_ms,
    })
}

pub(crate) async fn run_maker_bot(
    sender: Sender<CommandIntent>,
    config: MakerBotRuntimeConfig,
) -> Result<(), SendError<CommandIntent>> {
    for _round in 0..3 {
        sender
            .send(SubmitOrder {
                quantity: config.quantity,
                side: Side::Buy,
                order_type: Limit(config.bid_price),
            })
            .await?;

        sender
            .send(SubmitOrder {
                quantity: config.quantity,
                side: Side::Sell,
                order_type: Limit(config.ask_price),
            })
            .await?;

        sleep(Duration::from_millis(config.delay_ms)).await;
    }

    Ok(())
}
