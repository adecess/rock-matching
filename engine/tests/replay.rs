use rock_matching_engine::Command::{CancelOrder, SubmitOrder};
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::replay::{append_command, read_commands};
use rock_matching_engine::{
    ApplyError, Command, Engine, Event, OrderId, Price, Qty, Side, Timestamp,
};

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

    let (first_engine, first_events) = run(&commands);
    let (second_engine, second_events) = run(&commands);

    assert_eq!(first_events, second_events);
    assert_eq!(first_engine, second_engine);
}

#[test]
fn replay_through_log_preserves_state_and_events() {
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

    let (first_engine, first_events) = run(&commands);

    let mut buffer: Vec<u8> = Vec::new();
    for command in &commands {
        append_command(&mut buffer, command).unwrap();
    }

    let decoded_commands = read_commands(&buffer[..]).unwrap();
    let (second_engine, second_events) = run(&decoded_commands);

    assert_eq!(first_events, second_events);
    assert_eq!(first_engine, second_engine);
}
