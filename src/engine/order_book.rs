use std::collections::{BTreeMap, VecDeque};
use crate::engine::order::{Order, Price};

pub struct OrderBook {
    pub buy_orders: BTreeMap<Price, VecDeque<Order>>,
    pub sell_orders: BTreeMap<Price, VecDeque<Order>>,
}

impl OrderBook {
    pub fn new() -> OrderBook {
        OrderBook {
            buy_orders: BTreeMap::new(),
            sell_orders: BTreeMap::new(),
        }
    }
}
