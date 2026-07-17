use rock_matching_engine::{BookSnapshot, OrderId, OrderType, Price, Qty, Side};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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

pub(crate) struct MakerBotConfig {
    pub(crate) reference_price: Price,
    pub(crate) max_bid_distance: Price,
    pub(crate) max_ask_distance: Price,
    pub(crate) max_quantity: Qty,
    pub(crate) min_quantity: Qty,
    pub(crate) delay_ms: u64,
}

pub(crate) struct TakerBotConfig {
    pub(crate) min_quantity: Qty,
    pub(crate) max_quantity: Qty,
    pub(crate) delay_ms: u64,
    pub(crate) startup_delay_ms: u64,
}
