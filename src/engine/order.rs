use std::ops::Sub;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrderId(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Qty(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

// represents resting state on the book (which only limit orders achieve)
#[derive(Debug, Clone)]
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
