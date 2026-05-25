use crate::engine::order::{Order, OrderId, Price, Qty, Side};
use crate::engine::order_book::{Event, OrderBook};
use serde::{Deserialize, Serialize};

#[derive(Default, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct Timestamp(pub u64);

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum OrderType {
    Limit(Price),
    Market,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Command {
    SubmitOrder {
        timestamp: Timestamp,
        quantity: Qty,
        side: Side,
        order_type: OrderType,
    },
    CancelOrder {
        order_id: OrderId,
        timestamp: Timestamp,
    },
}

#[derive(Debug, PartialEq)]
pub enum ApplyError {
    OrderNotFound(OrderId),
    InvalidPrice(Price),
    InvalidQuantity(Qty),
    TimestampRegression,
}

#[derive(Default, PartialEq, Debug)]
pub struct Engine {
    order_book: OrderBook,
    next_order_id: u64,
    last_timestamp: Timestamp,
}

impl Engine {
    pub fn apply(&mut self, command: Command) -> Result<Vec<Event>, ApplyError> {
        match command {
            Command::SubmitOrder {
                timestamp,
                quantity,
                side,
                order_type,
            } => {
                self.check_and_update_timestamp(timestamp)?;
                Self::check_invalid_quantity(quantity)?;

                let order_id = self.assign_order_id();

                match order_type {
                    OrderType::Limit(price) => {
                        Self::check_invalid_price(price)?;
                        let incoming_order = Order::new(order_id, price, quantity, side);
                        Ok(self.order_book.match_limit_order(incoming_order))
                    }
                    OrderType::Market => {
                        Ok(self.order_book.match_market_order(order_id, side, quantity))
                    }
                }
            }
            Command::CancelOrder {
                order_id,
                timestamp,
            } => {
                self.check_and_update_timestamp(timestamp)?;

                self.order_book
                    .cancel_order(order_id)
                    .ok_or(ApplyError::OrderNotFound(order_id))
            }
        }
    }

    fn check_and_update_timestamp(&mut self, timestamp: Timestamp) -> Result<(), ApplyError> {
        if timestamp <= self.last_timestamp {
            return Err(ApplyError::TimestampRegression);
        }
        self.last_timestamp = timestamp;

        Ok(())
    }

    fn check_invalid_price(price: Price) -> Result<(), ApplyError> {
        if price == Price(0) {
            return Err(ApplyError::InvalidPrice(price));
        }

        Ok(())
    }

    fn check_invalid_quantity(quantity: Qty) -> Result<(), ApplyError> {
        if quantity == Qty(0) {
            return Err(ApplyError::InvalidQuantity(quantity));
        }

        Ok(())
    }

    fn assign_order_id(&mut self) -> OrderId {
        let id = OrderId(self.next_order_id);
        self.next_order_id = id.0 + 1;

        id
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::core::ApplyError::{
        InvalidPrice, InvalidQuantity, OrderNotFound, TimestampRegression,
    };
    use crate::engine::core::Command::{CancelOrder, SubmitOrder};
    use crate::engine::core::OrderType::{Limit, Market};
    use crate::engine::core::{Command, Engine, Timestamp};
    use crate::engine::order::{OrderId, Price, Qty, Side};
    use crate::engine::order_book::Event;

    #[test]
    fn limit_orders_are_successfully_submitted_and_trade() {
        let mut engine = Engine::default();
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(1),
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Limit(Price(102)),
            })
            .expect("Initial buy order submission failed");
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(2),
                quantity: Qty(3),
                side: Side::Buy,
                order_type: Limit(Price(100)),
            })
            .expect("Second buy order submission failed");
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(3),
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Limit(Price(100)),
            })
            .expect("Third buy order submission failed");

        let events = engine.apply(SubmitOrder {
            timestamp: Timestamp(4),
            quantity: Qty(10),
            side: Side::Sell,
            order_type: Limit(Price(100)),
        });

        assert_eq!(
            events,
            Ok(Vec::from([
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(0),
                    taker_side: Side::Sell,
                    price: Price(102),
                    quantity: Qty(1)
                },
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(1),
                    taker_side: Side::Sell,
                    price: Price(100),
                    quantity: Qty(3)
                },
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(2),
                    taker_side: Side::Sell,
                    price: Price(100),
                    quantity: Qty(1)
                },
                Event::OrderAddedToBook(OrderId(3), Side::Sell, Price(100), Qty(5),)
            ]))
        );
    }

    #[test]
    fn order_with_invalid_price_cannot_be_submitted() {
        let mut engine = Engine::default();

        let error = engine.apply(SubmitOrder {
            timestamp: Timestamp(4),
            quantity: Qty(15678),
            side: Side::Sell,
            order_type: Limit(Price(0)),
        });

        assert_eq!(error, Err(InvalidPrice(Price(0))));
    }

    #[test]
    fn order_with_invalid_quantity_cannot_be_submitted() {
        let mut engine = Engine::default();

        let error = engine.apply(SubmitOrder {
            timestamp: Timestamp(4),
            quantity: Qty(0),
            side: Side::Sell,
            order_type: Limit(Price(100)),
        });

        assert_eq!(error, Err(InvalidQuantity(Qty(0))));
    }

    #[test]
    fn market_buy_order_successfully_submitted_and_trades() {
        let mut engine = Engine::default();
        engine
            .apply(Command::SubmitOrder {
                timestamp: Timestamp(1),
                order_type: Limit(Price(99)),
                quantity: Qty(5),
                side: Side::Sell,
            })
            .expect("Sell order submission failed");

        let events = engine.apply(SubmitOrder {
            timestamp: Timestamp(2),
            order_type: Market,
            quantity: Qty(5),
            side: Side::Buy,
        });

        assert_eq!(
            events,
            Ok(vec![Event::OrderTraded {
                taker: OrderId(1),
                maker: OrderId(0),
                taker_side: Side::Buy,
                price: Price(99),
                quantity: Qty(5)
            }])
        );
    }

    #[test]
    fn engine_successfully_cancels_existing_order() {
        let mut engine = Engine::default();
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(1),
                quantity: Qty(1536),
                side: Side::Buy,
                order_type: Limit(Price(102)),
            })
            .expect("Initial buy order submission failed");
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(2),
                quantity: Qty(78),
                side: Side::Buy,
                order_type: Limit(Price(100)),
            })
            .expect("Second buy order submission failed");
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(3),
                quantity: Qty(9026),
                side: Side::Sell,
                order_type: Limit(Price(1000)),
            })
            .expect("First sell order submission failed");

        let events = engine.apply(CancelOrder {
            order_id: OrderId(1),
            timestamp: Timestamp(4),
        });

        assert_eq!(
            events,
            Ok(vec![Event::OrderCancelled {
                order_id: OrderId(1),
                cancelled_quantity: Qty(78)
            }])
        );
    }

    #[test]
    fn engine_returns_error_if_order_cancelled_does_not_exist() {
        let mut engine = Engine::default();
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(1),
                quantity: Qty(1536),
                side: Side::Buy,
                order_type: Limit(Price(102)),
            })
            .expect("Initial buy order submission failed");
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(2),
                quantity: Qty(78),
                side: Side::Buy,
                order_type: Limit(Price(100)),
            })
            .expect("Second buy order submission failed");
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(3),
                quantity: Qty(9026),
                side: Side::Sell,
                order_type: Limit(Price(1000)),
            })
            .expect("First sell order submission failed");

        let error = engine.apply(CancelOrder {
            order_id: OrderId(39018),
            timestamp: Timestamp(45678),
        });

        assert_eq!(error, Err(OrderNotFound(OrderId(39018))));
    }

    #[test]
    fn timestamp_error_is_returned_if_order_timestamp_is_lower_or_equal_to_last_timestamp() {
        let mut engine = Engine::default();
        engine
            .apply(SubmitOrder {
                timestamp: Timestamp(1),
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Limit(Price(101)),
            })
            .expect("Initial buy order submission failed");

        let timestamp_error = engine.apply(SubmitOrder {
            timestamp: Timestamp(1),
            quantity: Qty(1),
            side: Side::Buy,
            order_type: Limit(Price(101)),
        });

        assert_eq!(Err(TimestampRegression), timestamp_error);
    }
}
