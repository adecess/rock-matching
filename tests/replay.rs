use rock_matching::engine::core::Command::SubmitOrder;
use rock_matching::engine::core::OrderType::Limit;
use rock_matching::engine::core::{ApplyError, Engine, Timestamp};
use rock_matching::engine::order::{Price, Qty, Side};
use rock_matching::engine::order_book::Event;

#[test]
fn two_engines_with_the_same_submit_order_commands_produce_the_same_events() {
    let mut first_engine = Engine::default();
    let mut first_events: Vec<Result<Vec<Event>, ApplyError>> = Vec::new();

    first_events.push(first_engine.apply(SubmitOrder {
        timestamp: Timestamp(1),
        quantity: Qty(1),
        side: Side::Buy,
        order_type: Limit(Price(102)),
    }));
    first_events.push(first_engine.apply(SubmitOrder {
        timestamp: Timestamp(2),
        quantity: Qty(3),
        side: Side::Buy,
        order_type: Limit(Price(100)),
    }));
    first_events.push(first_engine.apply(SubmitOrder {
        timestamp: Timestamp(3),
        quantity: Qty(1),
        side: Side::Buy,
        order_type: Limit(Price(100)),
    }));

    let mut second_engine = Engine::default();
    let mut second_events: Vec<Result<Vec<Event>, ApplyError>> = Vec::new();

    second_events.push(second_engine.apply(SubmitOrder {
        timestamp: Timestamp(1),
        quantity: Qty(1),
        side: Side::Buy,
        order_type: Limit(Price(102)),
    }));
    second_events.push(second_engine.apply(SubmitOrder {
        timestamp: Timestamp(2),
        quantity: Qty(3),
        side: Side::Buy,
        order_type: Limit(Price(100)),
    }));
    second_events.push(second_engine.apply(SubmitOrder {
        timestamp: Timestamp(3),
        quantity: Qty(1),
        side: Side::Buy,
        order_type: Limit(Price(100)),
    }));

    assert_eq!(first_engine, second_engine);
    assert_eq!(first_events, second_events);
}
