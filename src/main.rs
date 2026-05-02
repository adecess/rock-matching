use rock_matching::engine::core::{Command, Engine, Timestamp};
use rock_matching::engine::core::OrderType::Limit;
use rock_matching::engine::order::{Price, Qty, Side};

fn main() {
    let mut engine = Engine::default();

    engine
        .apply(Command::SubmitOrder {
            quantity: Qty(0),
            side: Side::Buy,
            timestamp: Timestamp(1),
            order_type: Limit(Price(100)),
        })
        .expect("TODO: panic message");
}
