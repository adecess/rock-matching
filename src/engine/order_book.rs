use crate::engine::order::{Order, OrderId, Price, Qty, Side};
use std::cmp::Ordering;
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
    NoOrder,
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

    fn match_limit_order(&mut self, incoming_order: Order) -> Event {
        let order_id = incoming_order.order_id;
        let order_price = incoming_order.price;
        let order_quantity = incoming_order.quantity;

        match incoming_order.side {
            Side::Buy => loop {
                match self.sell_orders.first_entry() {
                    Some(price_level) => match price_level.key().cmp(&order_price) {
                        Ordering::Greater => {
                            self.add_to_book(order_price, Side::Buy, incoming_order);
                            break Event::OrderAddedToBook(
                                order_id,
                                Side::Buy,
                                order_price,
                                order_quantity,
                            );
                        }
                        _ => todo!(),
                    },
                    None => break Event::NoOrder,
                }
            },
            Side::Sell => {
                if let Some(price_level) = self.buy_orders.last_entry() {
                    match price_level.key().cmp(&order_price) {
                        Ordering::Less => {
                            self.add_to_book(order_price, Side::Sell, incoming_order);
                            Event::OrderAddedToBook(
                                order_id,
                                Side::Sell,
                                order_price,
                                order_quantity,
                            )
                        }
                        _ => todo!(),
                    }
                } else {
                    self.add_to_book(order_price, Side::Sell, incoming_order);
                    Event::OrderAddedToBook(order_id, Side::Sell, order_price, order_quantity)
                }
            }
        }
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
        let order = Order::new(order_id, Side::Sell, price, Qty(5));

        let event = order_book.match_limit_order(order);
        assert_eq!(
            event,
            Event::OrderAddedToBook(order_id, Side::Sell, price, Qty(5))
        );
        assert_eq!(order_book.sell_orders.len(), 1);
        assert_eq!(order_book.buy_orders.len(), 0);
    }
}
