use crate::engine::order::{OrderId, Price, Qty, Side};
use crate::engine::order_book::{Event, OrderBook};

#[derive(Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct Timestamp(pub u64);

pub enum Command {
    SubmitOrder {
        timestamp: Timestamp,
        price: Price,
        quantity: Qty,
        side: Side,
    },
    CancelOrder {
        timestamp: Timestamp,
        order_id: OrderId,
    },
}

#[derive(Debug)]
pub enum ApplyError {
    OrderNotFound(OrderId),
    InvalidPrice(Price),
    ZeroQuantity,
    TimeStampRegression,
}

#[derive(Default)]
pub struct Engine {
    order_book: OrderBook,
    last_timestamp: Timestamp,
}

impl Engine {
    pub fn apply(&self, command: Command) -> Result<Vec<Event>, ApplyError> {
        match command {
            Command::SubmitOrder { timestamp, .. } | Command::CancelOrder { timestamp, .. } => {
                if timestamp < self.last_timestamp {
                    return Err(ApplyError::TimeStampRegression);
                }
            }
        }

        
        Ok(vec![])
    }
}
