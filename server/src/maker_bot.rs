use rock_matching_engine::Command::SubmitOrder;
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{Command, Price, Qty, Side, Timestamp};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::{Duration, sleep};

pub(crate) async fn run_maker_bot(sender: Sender<Command>) -> Result<(), SendError<Command>> {
    let mut timestamp = 0;

    for _round in 0..3 {
        timestamp += 1;
        sender
            .send(SubmitOrder {
                timestamp: Timestamp(timestamp),
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Limit(Price(99)),
            })
            .await?;

        timestamp += 1;
        sender
            .send(SubmitOrder {
                timestamp: Timestamp(timestamp),
                quantity: Qty(1),
                side: Side::Sell,
                order_type: Limit(Price(101)),
            })
            .await?;

        sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
