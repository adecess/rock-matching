use crate::engine::order::{Order, Price};
use std::collections::{BTreeMap, VecDeque};

#[derive(Default)]
pub struct OrderBook {
    pub buy_orders: BTreeMap<Price, VecDeque<Order>>,
    pub sell_orders: BTreeMap<Price, VecDeque<Order>>,
}
