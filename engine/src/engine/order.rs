use serde::{Deserialize, Serialize};
use std::iter::Sum;
use std::ops::Sub;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrderId(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Price(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Qty(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Sub for Qty {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Sum for Qty {
    fn sum<I: Iterator<Item = Qty>>(iter: I) -> Qty {
        iter.fold(Qty(0), |acc, x| Qty(acc.0 + x.0))
    }
}

// represents resting state on the book (which only limit orders achieve)
#[derive(Debug, PartialEq)]
pub struct Order {
    pub order_id: OrderId,
    pub price: Price,
    pub quantity: Qty,
    pub side: Side,
}

impl Order {
    pub fn new(order_id: OrderId, price: Price, quantity: Qty, side: Side) -> Order {
        Order {
            order_id,
            price,
            quantity,
            side,
        }
    }
}
