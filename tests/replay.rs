use rock_matching::engine::core::Command::{CancelOrder, SubmitOrder};
use rock_matching::engine::core::OrderType::Limit;
use rock_matching::engine::core::{ApplyError, Command, Engine, Timestamp};
use rock_matching::engine::order::{OrderId, Price, Qty, Side};
use rock_matching::engine::order_book::Event;

fn run(commands: &[Command]) -> (Engine, Vec<Result<Vec<Event>, ApplyError>>) {
    let mut engine = Engine::default();
    let events = commands.iter().cloned().map(|c| engine.apply(c)).collect();
    (engine, events)
}

#[test]
fn two_engines_with_the_same_commands_reach_the_same_state() {
    let commands = vec![
        SubmitOrder {
            timestamp: Timestamp(1),
            quantity: Qty(1),
            side: Side::Buy,
            order_type: Limit(Price(102)),
        },
        SubmitOrder {
            timestamp: Timestamp(2),
            quantity: Qty(3),
            side: Side::Sell,
            order_type: Limit(Price(100)),
        },
        SubmitOrder {
            timestamp: Timestamp(3),
            quantity: Qty(1),
            side: Side::Buy,
            order_type: Limit(Price(100)),
        },
        CancelOrder {
            timestamp: Timestamp(4),
            order_id: OrderId(1),
        },
    ];

    let (first_engine, events_a) = run(&commands);
    let (second_engine, events_b) = run(&commands);

    assert_eq!(first_engine, second_engine);
    assert_eq!(events_a, events_b);
}
