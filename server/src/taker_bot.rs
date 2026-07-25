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
    use tokio::sync::mpsc::error::TryRecvError;

    fn valid_config() -> TakerBotConfig {
        TakerBotConfig {
            min_quantity: Qty(1),
            max_quantity: Qty(10),
            delay_ms: 20,
            startup_delay_ms: 10,
        }
    }

    #[test]
    fn accepts_valid_config() {
        let config =
            validate_taker_config(valid_config()).expect("valid taker config should be accepted");

        assert_eq!(config.min_quantity, Qty(1));
        assert_eq!(config.max_quantity, Qty(10));
        assert_eq!(config.delay_ms, 20);
        assert_eq!(config.startup_delay_ms, 10);
    }

    #[test]
    fn rejects_invalid_config() {
        let invalid_configs = [
            (
                TakerBotConfig {
                    min_quantity: Qty(0),
                    ..valid_config()
                },
                "taker min_quantity must be greater than zero",
            ),
            (
                TakerBotConfig {
                    min_quantity: Qty(2),
                    max_quantity: Qty(1),
                    ..valid_config()
                },
                "taker max_quantity must be greater than min_quantity",
            ),
            (
                TakerBotConfig {
                    delay_ms: 0,
                    ..valid_config()
                },
                "taker delay_ms must be greater than zero",
            ),
        ];

        for (config, expected_error) in invalid_configs {
            assert_eq!(
                validate_taker_config(config).err(),
                Some(expected_error),
                "taker config should be rejected with the expected error"
            );
        }
    }

    fn assert_submit_order(command: CommandIntent, expected_side: Side) {
        let SubmitOrder {
            quantity,
            side,
            order_type,
        } = command
        else {
            panic!("taker should submit an order");
        };

        assert_eq!(quantity, Qty(1));
        assert_eq!(side, expected_side);
        assert_eq!(order_type, Market);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_both_delays_and_alternates_order_sides() {
        let (sender, mut receiver) = mpsc::channel(2);
        let shutdown = CancellationToken::new();
        let bot = tokio::spawn(run_taker_bot(
            sender,
            TakerBotConfig {
                min_quantity: Qty(1),
                max_quantity: Qty(1),
                delay_ms: 20,
                startup_delay_ms: 10,
            },
            shutdown.clone(),
        ));

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(10)).await;
        tokio::task::yield_now().await;
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));

        tokio::time::advance(Duration::from_millis(20)).await;
        tokio::task::yield_now().await;
        assert_submit_order(
            receiver
                .try_recv()
                .expect("taker should submit an order after both delays"),
            Side::Buy,
        );

        tokio::time::advance(Duration::from_millis(20)).await;
        tokio::task::yield_now().await;
        assert_submit_order(
            receiver
                .try_recv()
                .expect("taker should submit another order after its delay"),
            Side::Sell,
        );

        shutdown.cancel();
        assert!(bot.await.expect("taker task should finish").is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn stops_without_sending_when_shutdown_during_startup_delay() {
        let (sender, mut receiver) = mpsc::channel(1);
        let shutdown = CancellationToken::new();
        let bot = tokio::spawn(run_taker_bot(
            sender,
            TakerBotConfig {
                min_quantity: Qty(1),
                max_quantity: Qty(1),
                delay_ms: 20,
                startup_delay_ms: 10,
            },
            shutdown.clone(),
        ));

        tokio::task::yield_now().await;
        shutdown.cancel();

        assert!(bot.await.expect("taker task should finish").is_ok());
        assert!(matches!(
            receiver.try_recv(),
            Err(TryRecvError::Disconnected)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn returns_error_when_command_receiver_is_closed() {
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let bot = tokio::spawn(run_taker_bot(
            sender,
            TakerBotConfig {
                min_quantity: Qty(1),
                max_quantity: Qty(1),
                delay_ms: 20,
                startup_delay_ms: 0,
            },
            CancellationToken::new(),
        ));

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(20)).await;
        tokio::task::yield_now().await;

        assert!(
            bot.await
                .expect("taker task should finish after send failure")
                .is_err()
        );
    }
}
