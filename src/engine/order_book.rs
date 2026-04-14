use crate::engine::order::{Order, Price, Side};
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

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

    fn match_limit_order(&mut self, incoming_order: Order) {
        let order_price = incoming_order.price;

        match incoming_order.side {
            Side::Buy => {
                if let Some((price, _price_list)) = self.sell_orders.first_key_value() {
                    match price.cmp(&order_price) {
                        Ordering::Greater => {
                            self.add_to_book(order_price, Side::Buy, incoming_order);
                        }
                        _ => todo!(),
                    }
                } else {
                    self.add_to_book(order_price, Side::Buy, incoming_order);
                }
            }
            Side::Sell => {
                if let Some((price, _price_list)) = self.buy_orders.last_key_value() {
                    match price.cmp(&order_price) {
                        Ordering::Less => {
                            self.add_to_book(order_price, Side::Sell, incoming_order);
                        }
                        _ => todo!(),
                    }
                } else {
                    self.add_to_book(order_price, Side::Sell, incoming_order);
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
        let order = Order::new(order_id, price, Qty(5), Side::Sell);

        order_book.match_limit_order(order);
        assert_eq!(order_book.sell_orders.len(), 1);
        assert_eq!(order_book.buy_orders.len(), 0);
    }
}
