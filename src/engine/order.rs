#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrderId(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Qty(pub u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

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
            quantity,
            price,
            side,
        }
    }
}
