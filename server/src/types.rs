use rock_matching_engine::{BookSnapshot, OrderId, OrderType, Price, Qty, Side};

#[derive(Debug, Clone)]
pub(crate) struct ServerEvent {
    pub(crate) snapshot: BookSnapshot,
    pub(crate) last_price: Option<Price>,
}

pub(crate) enum CommandIntent {
    SubmitOrder {
        quantity: Qty,
        side: Side,
        order_type: OrderType,
    },
    CancelOrder {
        order_id: OrderId,
    },
}
