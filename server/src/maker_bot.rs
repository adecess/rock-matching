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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::TryRecvError;

    fn valid_config() -> MakerBotConfig {
        MakerBotConfig {
            reference_price: Price(100),
            max_bid_distance: Price(10),
            max_ask_distance: Price(10),
            min_quantity: Qty(1),
            max_quantity: Qty(10),
            delay_ms: 20,
        }
    }

    #[test]
    fn accepts_valid_config() {
        let config =
            validate_maker_config(valid_config()).expect("valid maker config should be accepted");

        assert_eq!(config.reference_price, Price(100));
        assert_eq!(config.max_bid_distance, Price(10));
        assert_eq!(config.max_ask_distance, Price(10));
        assert_eq!(config.min_quantity, Qty(1));
        assert_eq!(config.max_quantity, Qty(10));
        assert_eq!(config.delay_ms, 20);
    }

    #[test]
    fn rejects_invalid_config() {
        let invalid_configs = [
            (
                MakerBotConfig {
                    max_bid_distance: Price(0),
                    ..valid_config()
                },
                "maker max_bid_distance must be greater than zero",
            ),
            (
                MakerBotConfig {
                    max_ask_distance: Price(0),
                    ..valid_config()
                },
                "maker max_ask_distance must be greater than zero",
            ),
            (
                MakerBotConfig {
                    min_quantity: Qty(0),
                    ..valid_config()
                },
                "maker min_quantity must be greater than zero",
            ),
            (
                MakerBotConfig {
                    min_quantity: Qty(2),
                    max_quantity: Qty(1),
                    ..valid_config()
                },
                "maker max_quantity must be greater than min_quantity",
            ),
            (
                MakerBotConfig {
                    delay_ms: 0,
                    ..valid_config()
                },
                "maker delay_ms must be greater than zero",
            ),
            (
                MakerBotConfig {
                    reference_price: Price(1),
                    max_bid_distance: Price(2),
                    ..valid_config()
                },
                "maker bid price would underflow",
            ),
            (
                MakerBotConfig {
                    reference_price: Price(u64::MAX),
                    max_ask_distance: Price(1),
                    ..valid_config()
                },
                "maker ask price would overflow",
            ),
        ];

        for (config, expected_error) in invalid_configs {
            assert_eq!(
                validate_maker_config(config).err(),
                Some(expected_error),
                "maker config should be rejected with the expected error"
            );
        }
    }

    fn assert_submit_order(
        command: CommandIntent,
        expected_quantity: Qty,
        expected_side: Side,
        expected_order_type: rock_matching_engine::OrderType,
    ) {
        let SubmitOrder {
            quantity,
            side,
            order_type,
        } = command
        else {
            panic!("maker should submit an order");
        };

        assert_eq!(quantity, expected_quantity);
        assert_eq!(side, expected_side);
        assert_eq!(order_type, expected_order_type);
    }

    #[tokio::test(start_paused = true)]
    async fn sends_bid_and_ask_each_cycle_and_stops_on_shutdown() {
        let (sender, mut receiver) = mpsc::channel(4);
        let shutdown = CancellationToken::new();
        let bot = tokio::spawn(run_maker_bot(
            sender,
            MakerBotConfig {
                reference_price: Price(100),
                max_bid_distance: Price(1),
                max_ask_distance: Price(1),
                min_quantity: Qty(2),
                max_quantity: Qty(2),
                delay_ms: 20,
            },
            shutdown.clone(),
        ));

        tokio::task::yield_now().await;

        assert_submit_order(
            receiver
                .try_recv()
                .expect("maker should submit a bid immediately"),
            Qty(2),
            Side::Buy,
            Limit(Price(99)),
        );
        assert_submit_order(
            receiver
                .try_recv()
                .expect("maker should submit an ask immediately"),
            Qty(2),
            Side::Sell,
            Limit(Price(101)),
        );
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));

        tokio::time::advance(Duration::from_millis(19)).await;
        tokio::task::yield_now().await;
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));

        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
        assert_submit_order(
            receiver
                .try_recv()
                .expect("maker should submit another bid after its delay"),
            Qty(2),
            Side::Buy,
            Limit(Price(99)),
        );
        assert_submit_order(
            receiver
                .try_recv()
                .expect("maker should submit another ask after its delay"),
            Qty(2),
            Side::Sell,
            Limit(Price(101)),
        );

        shutdown.cancel();
        assert!(bot.await.expect("maker task should finish").is_ok());
    }

    #[tokio::test]
    async fn returns_error_when_command_receiver_is_closed() {
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);

        let result = run_maker_bot(sender, valid_config(), CancellationToken::new()).await;

        assert!(result.is_err());
    }
}
