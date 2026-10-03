use crate::types::CommandIntent::SubmitOrder;
use crate::types::{CommandIntent, MakerBotConfig};
use rand::random_range;
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{OrderId, Price, Qty, Side};
use std::collections::VecDeque;
use std::error::Error;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

// One pair is refreshed each cycle, bounding both quote age and resting depth.
const MAX_QUOTE_PAIRS: usize = 10;
type MakerBotError = Box<dyn Error + Send + Sync>;

struct QuotePair {
    bid_order_id: Option<OrderId>,
    ask_order_id: Option<OrderId>,
}

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
) -> Result<(), MakerBotError> {
    let mut quotes = VecDeque::<QuotePair>::with_capacity(MAX_QUOTE_PAIRS);
    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            result = async {
                if quotes.len() == MAX_QUOTE_PAIRS {
                    let oldest = quotes.pop_front().expect("quote window is full");
                    for order_id in [oldest.bid_order_id, oldest.ask_order_id].into_iter().flatten() {
                        sender.send(CommandIntent::CancelOrder { order_id }).await?;
                    }
                }

                let bid_distance = random_range(1..=config.max_bid_distance.0);
                let ask_distance = random_range(1..=config.max_ask_distance.0);
                let bid_price = Price(config.reference_price.0 - bid_distance);
                let ask_price = Price(config.reference_price.0 + ask_distance);
                let quantity = Qty(random_range(config.min_quantity.0..=config.max_quantity.0));

                let bid_order_id = submit_quote(&sender, quantity, Side::Buy, bid_price).await?;
                let ask_order_id = submit_quote(&sender, quantity, Side::Sell, ask_price).await?;
                quotes.push_back(QuotePair { bid_order_id, ask_order_id });
                Ok::<(), MakerBotError>(())
            } => { result?; }
        }

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

async fn submit_quote(
    sender: &Sender<CommandIntent>,
    quantity: Qty,
    side: Side,
    price: Price,
) -> Result<Option<OrderId>, MakerBotError> {
    let (reply, response) = oneshot::channel();
    sender
        .send(SubmitOrder {
            quantity,
            side,
            order_type: Limit(price),
            reply: Some(reply),
        })
        .await?;
    Ok(response.await??)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rock_matching_engine::{ApplyError, Command, Engine, Event, OrderType, Timestamp};
    use std::collections::BTreeMap;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::TryRecvError;

    fn apply_and_check_bounds(
        engine: &mut Engine,
        resting: &mut BTreeMap<u64, Qty>,
        command: Command,
    ) -> Vec<Event> {
        let events = match engine.apply(command) {
            Ok(events) => events,
            Err(ApplyError::OrderNotFound(_)) => Vec::new(),
            Err(error) => panic!("unexpected engine error: {error}"),
        };
        // Track individual orders from real engine events, including partial fills.
        for event in &events {
            match *event {
                Event::OrderAddedToBook(id, _, _, quantity) => {
                    assert!(resting.insert(id.0, quantity).is_none());
                }
                Event::OrderTraded {
                    maker, quantity, ..
                } => {
                    let remaining = resting.get_mut(&maker.0).expect("traded maker should rest");
                    *remaining = *remaining - quantity;
                    if *remaining == Qty(0) {
                        resting.remove(&maker.0);
                    }
                }
                Event::OrderCancelled { order_id, .. } => {
                    resting.remove(&order_id.0);
                }
            }
        }
        assert!(
            resting.len() <= 2 * MAX_QUOTE_PAIRS,
            "resting order count must stay bounded"
        );
        // Inspect the complete book, not the frontend's truncated depth.
        let snapshot = engine.top_levels(usize::MAX);
        let bid_quantity: Qty = snapshot.bids.iter().map(|level| level.quantity).sum();
        let ask_quantity: Qty = snapshot.asks.iter().map(|level| level.quantity).sum();
        assert!(bid_quantity.0 <= 10 * MAX_QUOTE_PAIRS as u64);
        assert!(ask_quantity.0 <= 10 * MAX_QUOTE_PAIRS as u64);
        assert_eq!(
            resting.values().map(|qty| qty.0).sum::<u64>(),
            bid_quantity.0 + ask_quantity.0
        );
        events
    }

    async fn simulate_long_running_market(taker_quantity: Option<u64>) {
        let (sender, mut receiver) = mpsc::channel(4);
        let shutdown = CancellationToken::new();
        let bot = tokio::spawn(run_maker_bot(
            sender,
            MakerBotConfig {
                max_bid_distance: Price(1),
                max_ask_distance: Price(1),
                min_quantity: Qty(10),
                max_quantity: Qty(10),
                delay_ms: 100,
                ..valid_config()
            },
            shutdown.clone(),
        ));
        let mut engine = Engine::default();
        let mut resting = BTreeMap::new();
        let mut timestamp = 0;
        let mut already_filled_cancellations = 0;

        // 1,000 simulated seconds: 1,000 complete rotations of the quote window.
        for cycle in 0..10_000 {
            if cycle > 0 {
                tokio::time::advance(Duration::from_millis(100)).await;
            }
            tokio::task::yield_now().await;
            let command_count = if cycle < MAX_QUOTE_PAIRS { 2 } else { 4 };
            for _ in 0..command_count {
                timestamp += 1;
                let (command, reply) =
                    match receiver.try_recv().expect("maker should refresh quotes") {
                        SubmitOrder {
                            quantity,
                            side,
                            order_type,
                            reply,
                        } => (
                            Command::SubmitOrder {
                                timestamp: Timestamp(timestamp),
                                quantity,
                                side,
                                order_type,
                            },
                            reply,
                        ),
                        CommandIntent::CancelOrder { order_id } => (
                            Command::CancelOrder {
                                timestamp: Timestamp(timestamp),
                                order_id,
                            },
                            None,
                        ),
                    };
                let is_cancel = matches!(command, Command::CancelOrder { .. });
                let events = apply_and_check_bounds(&mut engine, &mut resting, command);
                if is_cancel && events.is_empty() {
                    already_filled_cancellations += 1;
                }
                if let Some(reply) = reply {
                    let id = events.iter().find_map(|event| match event {
                        Event::OrderAddedToBook(id, ..) => Some(*id),
                        _ => None,
                    });
                    assert!(reply.send(Ok(id)).is_ok());
                }
                tokio::task::yield_now().await;
            }
            assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
            let snapshot = engine.top_levels(usize::MAX);
            assert!(
                !snapshot.bids.is_empty(),
                "each cycle should replenish bids"
            );
            assert!(
                !snapshot.asks.is_empty(),
                "each cycle should replenish asks"
            );

            if let Some(quantity) = taker_quantity {
                timestamp += 1;
                apply_and_check_bounds(
                    &mut engine,
                    &mut resting,
                    Command::SubmitOrder {
                        timestamp: Timestamp(timestamp),
                        quantity: Qty(quantity),
                        side: if cycle % 2 == 0 {
                            Side::Buy
                        } else {
                            Side::Sell
                        },
                        order_type: OrderType::Market,
                    },
                );
                if quantity == 100 {
                    let snapshot = engine.top_levels(usize::MAX);
                    assert!(if cycle % 2 == 0 {
                        snapshot.asks.is_empty()
                    } else {
                        snapshot.bids.is_empty()
                    });
                }
            } else {
                let expected_pairs = (cycle + 1).min(MAX_QUOTE_PAIRS);
                assert_eq!(resting.len(), 2 * expected_pairs);
                assert_eq!(
                    resting.values().map(|qty| qty.0).sum::<u64>(),
                    20 * expected_pairs as u64
                );
            }
        }
        if taker_quantity == Some(100) {
            assert!(
                already_filled_cancellations > 0,
                "fully filled quotes should expire harmlessly"
            );
        }
        shutdown.cancel();
        assert!(bot.await.expect("maker should stop").is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn resting_orders_and_depth_stay_bounded_without_a_taker() {
        simulate_long_running_market(None).await;
    }

    #[tokio::test(start_paused = true)]
    async fn resting_orders_and_depth_stay_bounded_with_partial_fills() {
        simulate_long_running_market(Some(5)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn fully_filled_quotes_expire_and_both_sides_are_replenished() {
        simulate_long_running_market(Some(100)).await;
    }

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
        order_id: OrderId,
    ) {
        let SubmitOrder {
            quantity,
            side,
            order_type,
            reply,
        } = command
        else {
            panic!("maker should submit an order");
        };

        assert_eq!(quantity, expected_quantity);
        assert_eq!(side, expected_side);
        assert_eq!(order_type, expected_order_type);
        assert!(
            reply
                .expect("maker must request an order ID")
                .send(Ok(Some(order_id)))
                .is_ok()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn replaces_oldest_pair_before_quoting_and_stops_on_shutdown() {
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
        for cycle in 0..12 {
            if cycle == 1 {
                tokio::time::advance(Duration::from_millis(19)).await;
                tokio::task::yield_now().await;
                assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
                tokio::time::advance(Duration::from_millis(1)).await;
            } else if cycle > 1 {
                tokio::time::advance(Duration::from_millis(20)).await;
            }
            tokio::task::yield_now().await;

            if cycle >= MAX_QUOTE_PAIRS as u64 {
                // Cancel the oldest pair before submitting either replacement.
                for expected_id in [2 * (cycle - 10), 2 * (cycle - 10) + 1] {
                    let CommandIntent::CancelOrder { order_id } = receiver
                        .try_recv()
                        .expect("maker should cancel its oldest quote")
                    else {
                        panic!("maker must cancel before replacing");
                    };
                    assert_eq!(order_id, OrderId(expected_id));
                }
            }
            assert_submit_order(
                receiver.try_recv().expect("maker should submit a bid"),
                Qty(2),
                Side::Buy,
                Limit(Price(99)),
                OrderId(2 * cycle),
            );
            tokio::task::yield_now().await;
            assert_submit_order(
                receiver.try_recv().expect("maker should submit an ask"),
                Qty(2),
                Side::Sell,
                Limit(Price(101)),
                OrderId(2 * cycle + 1),
            );
            tokio::task::yield_now().await;
            assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        }

        shutdown.cancel();
        assert!(bot.await.expect("maker task should finish").is_ok());
    }

    #[tokio::test]
    async fn propagates_submission_errors_and_lost_replies() {
        for reject_submission in [false, true] {
            let (sender, mut receiver) = mpsc::channel(1);
            let bot = tokio::spawn(run_maker_bot(
                sender,
                valid_config(),
                CancellationToken::new(),
            ));
            let SubmitOrder { reply, .. } = receiver.recv().await.expect("maker should submit")
            else {
                panic!("expected submission");
            };
            let reply = reply.expect("maker should request a reply");
            if reject_submission {
                assert!(
                    reply
                        .send(Err(rock_matching_engine::ApplyError::InvalidPrice(Price(
                            0
                        ))))
                        .is_ok()
                );
            } else {
                drop(reply);
            }
            assert!(bot.await.expect("maker task should finish").is_err());
            assert!(matches!(
                receiver.try_recv(),
                Err(TryRecvError::Disconnected)
            ));
        }
    }

    #[tokio::test]
    async fn stops_while_waiting_for_a_submission_reply() {
        let (sender, mut receiver) = mpsc::channel(1);
        let shutdown = CancellationToken::new();
        let bot = tokio::spawn(run_maker_bot(sender, valid_config(), shutdown.clone()));
        let pending_command = receiver.recv().await.expect("maker should submit");
        shutdown.cancel();
        assert!(bot.await.expect("maker task should finish").is_ok());
        drop(pending_command);
    }

    #[tokio::test]
    async fn returns_error_when_command_receiver_is_closed() {
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);

        let result = run_maker_bot(sender, valid_config(), CancellationToken::new()).await;

        assert!(result.is_err());
    }
}
