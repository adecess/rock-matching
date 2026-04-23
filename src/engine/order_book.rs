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

        let order_id = incoming_order.order_id;
        let order_price = incoming_order.price;
        let remaining_order_quantity = incoming_order.quantity;
        let order_side = incoming_order.side;

        match order_side {
            Side::Buy => {
                while let Some(mut ask_price_level) = self.sell_orders.first_entry() {
                    match ask_price_level.key().cmp(&order_price) {
                        Ordering::Greater => {
                            self.add_to_book(order_price, Side::Buy, incoming_order);
                            events.push(Event::OrderAddedToBook(
                                order_id,
                                Side::Buy,
                                order_price,
                                remaining_order_quantity,
                            ));
                            break;
                        }
                        _ => {
                            let resting_order = ask_price_level.get_mut().front_mut().unwrap();

                            let trade_size = min(remaining_order_quantity, resting_order.quantity);

                            events.push(Event::OrderTraded {
                                taker: order_id,
                                maker: resting_order.order_id,
                                taker_side: Side::Buy,
                                price: order_price,
                                quantity: trade_size,
                            });

                            resting_order.quantity = resting_order.quantity - trade_size;
                            incoming_order.quantity = remaining_order_quantity - trade_size;

                            if resting_order.quantity.0 == 0 {
                                ask_price_level.get_mut().pop_front();
                            }

                            if ask_price_level.get_mut().is_empty() {
                                ask_price_level.remove();
                            }
                        }
                    }
                }
            }
            Side::Sell => {
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
    fn limit_order_is_added_to_order_book_if_there_is_no_matching_order() {
        let mut order_book = OrderBook::default();
        let order_id = OrderId(0);
        let price = Price(100);
        let order = Order::new(order_id, price, Qty(5), Side::Sell);

        let event = order_book.match_limit_order(order);
        assert_eq!(
            event[0],
            Event::OrderAddedToBook(order_id, Side::Sell, price, Qty(5))
        );
        assert_eq!(order_book.sell_orders.len(), 1);
        assert_eq!(order_book.buy_orders.len(), 0);
    }
}
