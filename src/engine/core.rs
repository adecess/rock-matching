use crate::engine::order::{OrderId, Price, Qty, Side};
use crate::engine::order_book::{Event, OrderBook};

pub enum Command {
    SubmitOrder {
        price: Price,
        quantity: Qty,
        side: Side,
    },
    CancelOrder(OrderId),
}

#[derive(Debug)]
pub enum ApplyError {
    OrderNotFound(OrderId),
    InvalidPrice(Price),
    ZeroQuantity,
}

#[derive(Default)]
pub struct Engine {
    order_book: OrderBook,
}

impl Engine {
    pub fn apply(&self, _command: Command) -> Result<Vec<Event>, ApplyError> {
        Ok(vec![])
    }
}
