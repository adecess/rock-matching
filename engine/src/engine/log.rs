use crate::engine::core::Command;
use std::io::{BufRead, BufReader, Read, Write};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LogError {
    #[error("log I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("log JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn append_command<W: Write>(writer: &mut W, command: &Command) -> Result<(), LogError> {
    let line = serde_json::to_string(command)?;
    writeln!(writer, "{}", line)?;
    Ok(())
}

pub fn read_commands<R: Read>(reader: R) -> Result<Vec<Command>, LogError> {
    let reader = BufReader::new(reader);
    reader
        .lines()
        .map(|line| -> Result<Command, LogError> {
            let line = line?;
            Ok(serde_json::from_str(&line)?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::engine::core::Command::{CancelOrder, SubmitOrder};
    use crate::engine::core::OrderType::Limit;
    use crate::engine::core::{Command, Timestamp};
    use crate::engine::log::{LogError, append_command, read_commands};
    use crate::engine::order::{OrderId, Price, Qty, Side};
    use std::error::Error as _;

    fn assert_std_error<T: std::error::Error>() {}

    #[test]
    fn log_errors_are_standard_errors_with_sources() {
        assert_std_error::<LogError>();

        let io_source = std::io::Error::other("disk full");
        let expected_io_message = format!("log I/O error: {io_source}");
        let io_error = LogError::from(io_source);

        assert_eq!(io_error.to_string(), expected_io_message);
        assert!(io_error.source().is_some());

        let json_source =
            serde_json::from_str::<Command>("{").expect_err("invalid JSON should be rejected");
        let expected_json_message = format!("log JSON parse error: {json_source}");
        let json_error = LogError::from(json_source);

        assert_eq!(json_error.to_string(), expected_json_message);
        assert!(json_error.source().is_some());
    }

    #[test]
    fn commands_format_is_consistent_after_encoding_and_decoding_back() {
        let commands = vec![
            SubmitOrder {
                timestamp: Timestamp(1),
                quantity: Qty(3),
                side: Side::Buy,
                order_type: Limit(Price(100)),
            },
            CancelOrder {
                order_id: OrderId(1),
                timestamp: Timestamp(2),
            },
        ];

        let mut buffer: Vec<u8> = Vec::new();
        for command in &commands {
            append_command(&mut buffer, command).unwrap();
        }

        let decoded = read_commands(&buffer[..]).unwrap();

        assert_eq!(commands, decoded);
    }
}
