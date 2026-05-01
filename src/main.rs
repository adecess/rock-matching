use rock_matching::engine::core::{Command, Engine};
use rock_matching::engine::order::{Price, Qty, Side};

fn main() {
    let engine = Engine::default();

    engine
        .apply(Command::SubmitOrder {
            price: Price(0),
            quantity: Qty(0),
            side: Side::Buy,
        })
        .expect("TODO: panic message");
}
