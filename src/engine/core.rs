use crate::engine::order::{Order, OrderId, Price, Qty, Side};
use crate::engine::order_book::{Event, OrderBook};

#[derive(Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct Timestamp(pub u64);

pub enum OrderType {
    Limit(Price),
    Market,
}

pub enum Command {
    SubmitOrder {
        timestamp: Timestamp,
        quantity: Qty,
        side: Side,
        order_type: OrderType,
    },
}

#[derive(Debug, PartialEq)]
pub enum ApplyError {
    OrderNotFound(OrderId),
    InvalidPrice(Price),
    ZeroQuantity,
    TimestampRegression,
}

#[derive(Default)]
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
                if timestamp <= self.last_timestamp {
                    return Err(ApplyError::TimestampRegression);
                }
                self.last_timestamp = timestamp;

                let order_id = self.assign_order_id();

                match order_type {
                    OrderType::Limit(price) => {
                        let incoming_order = Order::new(order_id, price, quantity, side);
                        Ok(self.order_book.match_limit_order(incoming_order))
                    }
                    OrderType::Market => {
                        Ok(self.order_book.match_market_order(order_id, side, quantity))
                    }
                }
            }
        }
    }

    fn assign_order_id(&mut self) -> OrderId {
        let id = OrderId(self.next_order_id);
        self.next_order_id = id.0 + 1;

        id
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::core::ApplyError::TimestampRegression;
    use crate::engine::core::Command::SubmitOrder;
    use crate::engine::core::OrderType::Limit;
    use crate::engine::core::{Engine, Timestamp};
    use crate::engine::order::{OrderId, Price, Qty, Side};
    use crate::engine::order_book::Event;

    #[test]
    fn limit_orders_are_successfully_submitted() {
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

        let events = engine
            .apply(SubmitOrder {
                timestamp: Timestamp(4),
                quantity: Qty(10),
                side: Side::Sell,
                order_type: Limit(Price(100)),
            })
            .expect("Incoming sell order submission failed");

        assert_eq!(
            events,
            [
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
            ]
        );
        assert_eq!(engine.order_book.buy_orders.len(), 0);
        assert_eq!(engine.order_book.sell_orders.len(), 1);
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
