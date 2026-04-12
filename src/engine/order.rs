#[derive(Debug, Copy, Clone)]
pub struct Price(i64);
#[derive(Debug, Copy, Clone)]
pub struct Qty(i64);
#[derive(Debug, Copy, Clone)]
pub enum Side {
    Buy,
    Sell,
}

pub struct Order {
    price: Price,
    quantity: Qty,
    side: Side,
}
