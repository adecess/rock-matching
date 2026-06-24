use crate::types::CommandIntent;
use crate::types::CommandIntent::SubmitOrder;
use rock_matching_engine::OrderType::Limit;
use rock_matching_engine::{Price, Qty, Side};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::{Duration, sleep};

pub(crate) async fn run_maker_bot(
    sender: Sender<CommandIntent>,
) -> Result<(), SendError<CommandIntent>> {
    for _round in 0..3 {
        sender
            .send(SubmitOrder {
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Limit(Price(99)),
            })
            .await?;

        sender
            .send(SubmitOrder {
                quantity: Qty(1),
                side: Side::Sell,
                order_type: Limit(Price(101)),
            })
            .await?;

        sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
