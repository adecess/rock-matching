use rock_matching::engine::core::{Command, Engine, Timestamp};
use rock_matching::engine::order::{Price, Qty, Side};

fn main() {
    let mut engine = Engine::default();

    engine
        .apply(Command::SubmitOrder {
            price: Price(0),
            quantity: Qty(0),
            side: Side::Buy,
            timestamp: Timestamp(1),
        })
        .expect("TODO: panic message");
}
