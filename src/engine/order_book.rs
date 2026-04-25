use crate::engine::order::{Order, OrderId, Price, Qty, Side};
use std::cmp::{Ordering, min};
use std::collections::{BTreeMap, VecDeque};
use std::ops::Sub;

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    OrderTraded {
        taker: OrderId,
        maker: OrderId,
        taker_side: Side,
        price: Price,
        quantity: Qty,
    },
    OrderAddedToBook(OrderId, Side, Price, Qty),
    OrderCancelled(OrderId),
}

#[derive(Default)]
pub struct OrderBook {
    pub buy_orders: BTreeMap<Price, VecDeque<Order>>,
    pub sell_orders: BTreeMap<Price, VecDeque<Order>>,
}

impl OrderBook {
    fn add_to_book(&mut self, price: Price, side: Side, incoming_order: Order) {
        match side {
            Side::Buy => {
                self.buy_orders
                    .entry(price)
                    .or_default()
                    .push_back(incoming_order);
            }
            Side::Sell => {
                self.sell_orders
                    .entry(price)
                    .or_default()
                    .push_back(incoming_order);
            }
        }
    }

    fn match_limit_order(&mut self, mut incoming_order: Order) -> Vec<Event> {
        let mut events = Vec::new();

        match incoming_order.side {
            Side::Buy => {
                while incoming_order.quantity > Qty(0)
                    && let Some(mut ask_price_level) = self.sell_orders.first_entry()
                {
                    match ask_price_level.key().cmp(&incoming_order.price) {
                        Ordering::Greater => {
                            break;
                        }
                        _ => {
                            let resting_order = ask_price_level.get_mut().front_mut().unwrap();
                            let trade_size = min(incoming_order.quantity, resting_order.quantity);

                            events.push(Event::OrderTraded {
                                taker: incoming_order.order_id,
                                maker: resting_order.order_id,
                                taker_side: Side::Buy,
                                price: resting_order.price,
                                quantity: trade_size,
                            });

                            resting_order.quantity = resting_order.quantity - trade_size;
                            incoming_order.quantity = incoming_order.quantity - trade_size;

                            if resting_order.quantity == Qty(0) {
                                ask_price_level.get_mut().pop_front();

                                if ask_price_level.get_mut().is_empty() {
                                    ask_price_level.remove();
                                }
                            }
                        }
                    }
                }

                let order_id = incoming_order.order_id;
                let order_price = incoming_order.price;
                let remaining_order_quantity = incoming_order.quantity;
                if remaining_order_quantity > Qty(0) {
                    self.add_to_book(incoming_order.price, Side::Buy, incoming_order);
                    events.push(Event::OrderAddedToBook(
                        order_id,
                        Side::Buy,
                        order_price,
                        remaining_order_quantity,
                    ));
                }
            }
            Side::Sell => {
                let order_id = incoming_order.order_id;
                let order_price = incoming_order.price;
                let remaining_order_quantity = incoming_order.quantity;
                if let Some(bid_price_level) = self.buy_orders.last_entry() {
                    match bid_price_level.key().cmp(&order_price) {
                        Ordering::Less => {
                            self.add_to_book(order_price, Side::Sell, incoming_order);
                            events.push(Event::OrderAddedToBook(
                                order_id,
                                Side::Sell,
                                order_price,
                                remaining_order_quantity,
                            ));
                        }
                        _ => todo!(),
                    }
                } else {
                    self.add_to_book(order_price, Side::Sell, incoming_order);
                    events.push(Event::OrderAddedToBook(
                        order_id,
                        Side::Sell,
                        order_price,
                        remaining_order_quantity,
                    ));
                }
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::order::{OrderId, Qty};

    #[test]
    fn limit_sell_order_is_added_to_order_book_if_there_is_no_matching_order() {
        let mut order_book = OrderBook::default();
        let order_id = OrderId(0);
        let price = Price(100);
        let order = Order::new(order_id, price, Qty(5), Side::Sell);

        let events = order_book.match_limit_order(order);
        assert_eq!(
            events.first(),
            Some(&Event::OrderAddedToBook(
                order_id,
                Side::Sell,
                price,
                Qty(5)
            ))
        );
        assert_eq!(order_book.sell_orders.len(), 1);
        assert_eq!(order_book.buy_orders.len(), 0);
    }

    #[test]
    fn limit_buy_order_trades_if_there_is_a_perfectly_matching_sell_order() {
        let mut order_book = OrderBook::default();
        order_book.sell_orders.insert(
            Price(99),
            VecDeque::from([Order::new(OrderId(0), Price(99), Qty(5), Side::Sell)]),
        );

        let incoming_buy_order = Order::new(OrderId(1), Price(100), Qty(5), Side::Buy);

        let events = order_book.match_limit_order(incoming_buy_order);
        assert_eq!(
            events[0],
            Event::OrderTraded {
                taker: OrderId(1),
                maker: OrderId(0),
                taker_side: Side::Buy,
                price: Price(99),
                quantity: Qty(5)
            }
        );
        assert_eq!(order_book.sell_orders.len(), 0);
        assert_eq!(order_book.buy_orders.len(), 0);
    }

    #[test]
    fn limit_buy_order_trades_if_there_are_enough_matching_sell_orders_at_the_same_price_level() {
        let mut order_book = OrderBook::default();
        order_book.sell_orders.insert(
            Price(99),
            VecDeque::from([
                Order::new(OrderId(0), Price(99), Qty(3), Side::Sell),
                Order::new(OrderId(1), Price(99), Qty(2), Side::Sell),
            ]),
        );

        let incoming_buy_order = Order::new(OrderId(2), Price(100), Qty(5), Side::Buy);

        let events = order_book.match_limit_order(incoming_buy_order);
        assert_eq!(
            events,
            [
                Event::OrderTraded {
                    taker: OrderId(2),
                    maker: OrderId(0),
                    taker_side: Side::Buy,
                    price: Price(99),
                    quantity: Qty(3)
                },
                Event::OrderTraded {
                    taker: OrderId(2),
                    maker: OrderId(1),
                    taker_side: Side::Buy,
                    price: Price(99),
                    quantity: Qty(2)
                }
            ]
        );
        assert_eq!(order_book.sell_orders.len(), 0);
        assert_eq!(order_book.buy_orders.len(), 0);
    }

    #[test]
    fn limit_buy_order_trades_if_there_are_enough_matching_sell_orders_at_different_price_levels() {
        let mut order_book = OrderBook::default();
        order_book.sell_orders.insert(
            Price(99),
            VecDeque::from([
                Order::new(OrderId(0), Price(99), Qty(1), Side::Sell),
                Order::new(OrderId(1), Price(99), Qty(2), Side::Sell),
                Order::new(OrderId(2), Price(100), Qty(2), Side::Sell),
            ]),
        );

        let incoming_buy_order = Order::new(OrderId(3), Price(100), Qty(5), Side::Buy);

        let events = order_book.match_limit_order(incoming_buy_order);
        assert_eq!(
            events,
            [
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(0),
                    taker_side: Side::Buy,
                    price: Price(99),
                    quantity: Qty(1)
                },
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(1),
                    taker_side: Side::Buy,
                    price: Price(99),
                    quantity: Qty(2)
                },
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(2),
                    taker_side: Side::Buy,
                    price: Price(100),
                    quantity: Qty(2)
                }
            ]
        );
        assert_eq!(order_book.sell_orders.len(), 0);
        assert_eq!(order_book.buy_orders.len(), 0);
    }

    #[test]
    fn limit_buy_order_trades_partially_if_there_are_not_enough_matching_sell_orders_at_different_price_levels()
     {
        let mut order_book = OrderBook::default();
        order_book.sell_orders.insert(
            Price(99),
            VecDeque::from([
                Order::new(OrderId(0), Price(99), Qty(1), Side::Sell),
                Order::new(OrderId(1), Price(99), Qty(1), Side::Sell),
                Order::new(OrderId(2), Price(100), Qty(1), Side::Sell),
            ]),
        );

        let incoming_buy_order = Order::new(OrderId(3), Price(100), Qty(5), Side::Buy);

        let events = order_book.match_limit_order(incoming_buy_order);
        assert_eq!(
            events,
            [
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(0),
                    taker_side: Side::Buy,
                    price: Price(99),
                    quantity: Qty(1)
                },
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(1),
                    taker_side: Side::Buy,
                    price: Price(99),
                    quantity: Qty(1)
                },
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(2),
                    taker_side: Side::Buy,
                    price: Price(100),
                    quantity: Qty(1)
                },
                Event::OrderAddedToBook(OrderId(3), Side::Buy, Price(100), Qty(2),)
            ]
        );
        assert_eq!(order_book.sell_orders.len(), 0);
        assert_eq!(order_book.buy_orders.len(), 1);
    }
}
