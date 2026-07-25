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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test(start_paused = true)]
    async fn sends_an_order_when_startup_delay_is_shorter_than_order_delay() {
        let (sender, mut receiver) = mpsc::channel(1);
        let shutdown = CancellationToken::new();
        let bot_shutdown = shutdown.clone();
        let bot = tokio::spawn(run_taker_bot(
            sender,
            TakerBotConfig {
                min_quantity: Qty(1),
                max_quantity: Qty(1),
                delay_ms: 20,
                startup_delay_ms: 10,
            },
            bot_shutdown,
        ));

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(10)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(20)).await;
        tokio::task::yield_now().await;

        let command = receiver
            .try_recv()
            .expect("taker should submit an order after both delays");
        assert!(matches!(
            command,
            SubmitOrder {
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Market,
            }
        ));

        shutdown.cancel();
        assert!(bot.await.expect("taker task should finish").is_ok());
    }
}
