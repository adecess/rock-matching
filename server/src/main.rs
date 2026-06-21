use rock_matching_engine::Command::SubmitOrder;
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{Command, Engine, Price, Qty, Side, Timestamp};
use tokio::sync::mpsc::channel;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = channel::<Command>(100);
    let mut engine = Engine::default();
    let handle = tokio::spawn(async move {
        while let Some(command) = rx.recv().await {
            let events = engine.apply(command).unwrap();

            println!("{events:?}");
        }
    });

    tx.send(SubmitOrder {
        timestamp: Timestamp(1),
        quantity: Qty(1),
        side: Side::Buy,
        order_type: Limit(Price(102)),
    })
        .await
        .unwrap();

    tx.send(SubmitOrder {
        timestamp: Timestamp(2),
        quantity: Qty(1),
        side: Side::Sell,
        order_type: Limit(Price(101)),
    })
        .await
        .unwrap();

    drop(tx);
    handle.await.unwrap();
}
