#[derive(Debug, Copy, Clone)]
pub struct Price(u64);
#[derive(Debug, Copy, Clone)]
pub struct Qty(u64);
#[derive(Debug, Copy, Clone)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct Order {
    order_id: u64,
    price: Price,
    quantity: Qty,
    side: Side,
}

impl Order {
    pub fn new(
        order_id: u64,
        price: Price,
        quantity: Qty,
        side: Side,
    ) -> Order {
        Order {
            order_id,
            quantity,
            price,
            side,
        }
    }
}
