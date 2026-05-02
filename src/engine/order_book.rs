use crate::engine::order::{Order, OrderId, Price, Qty, Side};
use std::cmp::{Ordering, min};
use std::collections::btree_map::OccupiedEntry;
use std::collections::{BTreeMap, VecDeque};

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

    fn pop_price_level(mut price_level: OccupiedEntry<Price, VecDeque<Order>>) {
        price_level.get_mut().pop_front();
        if price_level.get().is_empty() {
            price_level.remove();
        }
    }

    pub fn match_limit_order(&mut self, mut incoming_order: Order) -> Vec<Event> {
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
                            let resting_order = ask_price_level.get_mut().front_mut().expect(
                                "There should be at least one order at this ask price level",
                            );
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
                                Self::pop_price_level(ask_price_level);
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
                while let Some(mut bid_price_level) = self.buy_orders.last_entry() {
                    match bid_price_level.key().cmp(&incoming_order.price) {
                        Ordering::Less => {
                            break;
                        }
                        _ => {
                            let resting_order = bid_price_level.get_mut().front_mut().expect(
                                "There should be at least one order at this bid price level",
                            );
                            let trade_size = min(incoming_order.quantity, resting_order.quantity);

                            events.push(Event::OrderTraded {
                                taker: incoming_order.order_id,
                                maker: resting_order.order_id,
                                taker_side: Side::Sell,
                                price: resting_order.price,
                                quantity: trade_size,
                            });

                            incoming_order.quantity = incoming_order.quantity - trade_size;
                            resting_order.quantity = resting_order.quantity - trade_size;

                            if resting_order.quantity == Qty(0) {
                                Self::pop_price_level(bid_price_level);
                            }
                        }
                    }
                }

                let order_id = incoming_order.order_id;
                let order_price = incoming_order.price;
                let remaining_order_quantity = incoming_order.quantity;
                if remaining_order_quantity > Qty(0) {
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
    fn limit_sell_order_and_limit_buy_order_both_rest_on_order_book_if_they_do_not_match() {
        let mut order_book = OrderBook::default();
        let sell_order = Order::new(OrderId(0), Price(100), Qty(5), Side::Sell);
        let buy_order = Order::new(OrderId(1), Price(99), Qty(5), Side::Buy);

        let sell_order_added = order_book.match_limit_order(sell_order);
        assert_eq!(
            sell_order_added.first(),
            Some(&Event::OrderAddedToBook(
                OrderId(0),
                Side::Sell,
                Price(100),
                Qty(5)
            ))
        );

        let buy_order_added = order_book.match_limit_order(buy_order);
        assert_eq!(
            buy_order_added.first(),
            Some(&Event::OrderAddedToBook(
                OrderId(1),
                Side::Buy,
                Price(99),
                Qty(5)
            ))
        );
        assert_eq!(order_book.sell_orders.len(), 1);
        assert_eq!(order_book.buy_orders.len(), 1);
    }

    #[test]
    fn limit_buy_order_trades_if_there_is_a_perfectly_matching_sell_order() {
        let mut order_book = OrderBook::default();
        order_book.match_limit_order(Order::new(OrderId(0), Price(99), Qty(5), Side::Sell));

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
    fn limit_buy_order_trades_with_partial_maker_fill_if_there_is_a_matching_sell_order_with_higher_quantity()
     {
        let mut order_book = OrderBook::default();
        order_book.match_limit_order(Order::new(OrderId(0), Price(99), Qty(8), Side::Sell));

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

        let price_level = order_book
            .sell_orders
            .get(&Price(99))
            .expect("price level should exist");
        assert_eq!(order_book.sell_orders.len(), 1);
        assert_eq!(price_level.len(), 1);
        assert_eq!(price_level[0].quantity, Qty(3));
        assert_eq!(order_book.buy_orders.len(), 0);
    }

    #[test]
    fn limit_buy_order_trades_if_there_are_enough_matching_sell_orders_at_the_same_price_level() {
        let mut order_book = OrderBook::default();
        order_book.match_limit_order(Order::new(OrderId(0), Price(99), Qty(3), Side::Sell));
        order_book.match_limit_order(Order::new(OrderId(1), Price(99), Qty(2), Side::Sell));

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
        order_book.match_limit_order(Order::new(OrderId(0), Price(99), Qty(1), Side::Sell));
        order_book.match_limit_order(Order::new(OrderId(1), Price(99), Qty(2), Side::Sell));
        order_book.match_limit_order(Order::new(OrderId(2), Price(100), Qty(2), Side::Sell));

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
        order_book.match_limit_order(Order::new(OrderId(0), Price(99), Qty(1), Side::Sell));
        order_book.match_limit_order(Order::new(OrderId(1), Price(99), Qty(1), Side::Sell));
        order_book.match_limit_order(Order::new(OrderId(2), Price(100), Qty(1), Side::Sell));

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

    #[test]
    fn limit_sell_order_trades_partially_if_there_are_not_enough_matching_buy_orders_at_different_price_levels()
     {
        let mut order_book = OrderBook::default();
        order_book.match_limit_order(Order::new(OrderId(0), Price(101), Qty(1), Side::Buy));
        order_book.match_limit_order(Order::new(OrderId(1), Price(100), Qty(3), Side::Buy));
        order_book.match_limit_order(Order::new(OrderId(2), Price(100), Qty(1), Side::Buy));

        let incoming_sell_order = Order::new(OrderId(3), Price(100), Qty(10), Side::Sell);

        let events = order_book.match_limit_order(incoming_sell_order);
        assert_eq!(
            events,
            [
                Event::OrderTraded {
                    taker: OrderId(3),
                    maker: OrderId(0),
                    taker_side: Side::Sell,
                    price: Price(101),
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
        assert_eq!(order_book.buy_orders.len(), 0);
        assert_eq!(order_book.sell_orders.len(), 1);
    }
}
