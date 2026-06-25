use crate::types::CommandIntent;
use crate::types::CommandIntent::SubmitOrder;
use rock_matching_engine::OrderType::Market;
use rock_matching_engine::{Qty, Side};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::sleep;

pub(crate) async fn run_taker_bot(
    sender: Sender<CommandIntent>,
) -> Result<(), SendError<CommandIntent>> {
    loop {
        sleep(Duration::from_millis(1000)).await;

        sender
            .send(SubmitOrder {
                quantity: Qty(1),
                side: Side::Buy,
                order_type: Market,
            })
            .await?;
    }
}
