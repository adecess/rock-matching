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

pub(crate) struct MakerBotConfig {
    pub(crate) reference_price: Price,
    pub(crate) half_spread: Price,
    pub(crate) quantity: Qty,
    pub(crate) delay_ms: u64,
}

pub(crate) struct MakerBotRuntimeConfig {
    pub(crate) bid_price: Price,
    pub(crate) ask_price: Price,
    pub(crate) quantity: Qty,
    pub(crate) delay_ms: u64,
}

pub(crate) struct TakerBotConfig {
    pub(crate) quantity: Qty,
    pub(crate) delay_ms: u64,
}
